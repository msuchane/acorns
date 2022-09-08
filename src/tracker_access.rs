/*
cizrna: Generate an AsciiDoc release notes document from tracking tickets.
Copyright (C) 2022  Marek Suchánek  <msuchane@redhat.com>

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use std::collections::HashMap;
use std::string::ToString;
use std::sync::Arc;

use bugzilla_query::Bug;
use color_eyre::eyre::{bail, eyre, Result, WrapErr};
use jira_query::Issue;

// use crate::config::tracker::Service;
use crate::config::{tracker, QueryUsing, TicketQuery};
use crate::ticket_abstraction::{AbstractTicket, IntoAbstract};

/// The number of items in a single Jira query.
/// All Jira queries are processed in chunks of this size.
/// This prevents hitting the maximum allowed request size set in the Jira instance.
// TODO: Make this configurable.
const JIRA_CHUNK_SIZE: u32 = 30;

/// Always include these fields in Bugzilla requests. We process some of their content.
const BZ_INCLUDED_FIELDS: &[&str; 3] = &["_default", "pool", "flags"];

/// The environment variable that holds the API key to Bugzilla.
const BZ_API_KEY_VAR: &str = "BZ_API_KEY";

/// The environment variable that holds the API key to Jira.
const JIRA_API_KEY_VAR: &str = "JIRA_API_KEY";

#[derive(Clone)]
pub struct AnnotatedTicket {
    pub ticket: AbstractTicket,
    pub query: Arc<TicketQuery>,
}

impl AnnotatedTicket {
    /// Modify the ticket by applying the overrides configured for it.
    /// The overrides might edit several specific fields of `AbstractTicket`.
    pub fn override_fields(&mut self) {
        // The overrides configuration entry is optional.
        if let Some(overrides) = &self.query.overrides {
            // Each part of the overrides is also optional.
            if let Some(doc_type) = &overrides.doc_type {
                self.ticket.doc_type = doc_type.clone();
            }
            if let Some(components) = &overrides.components {
                self.ticket.components = components.clone();
            }
            if let Some(subsystems) = &overrides.subsystems {
                self.ticket.subsystems = subsystems.clone();
            }
        }
    }
}

/// Prepare a client to access Bugzilla.
fn bz_instance(trackers: &tracker::Config) -> Result<bugzilla_query::BzInstance> {
    let api_key = if let Some(key) = &trackers.bugzilla.api_key {
        key.clone()
    } else {
        // TODO: Store the name of the variable in a constant, or make it configurable.
        std::env::var(BZ_API_KEY_VAR)
            .wrap_err_with(|| format!("Set the {} environment variable.", BZ_API_KEY_VAR))?
    };

    Ok(
        bugzilla_query::BzInstance::at(trackers.bugzilla.host.clone())?
            .authenticate(bugzilla_query::Auth::ApiKey(api_key))?
            .paginate(bugzilla_query::Pagination::Unlimited)
            .include_fields(BZ_INCLUDED_FIELDS.iter().map(ToString::to_string).collect()),
    )
}
/// Prepare a client to access Jira.
fn jira_instance(trackers: &tracker::Config) -> Result<jira_query::JiraInstance> {
    let api_key = if let Some(key) = &trackers.jira.api_key {
        key.clone()
    } else {
        // TODO: Store the name of the variable in a constant, or make it configurable.
        std::env::var(JIRA_API_KEY_VAR)
            .wrap_err_with(|| format!("Set the {} environment variable.", JIRA_API_KEY_VAR))?
    };

    Ok(jira_query::JiraInstance::at(trackers.jira.host.clone())?
        .authenticate(jira_query::Auth::ApiKey(api_key))?
        .paginate(jira_query::Pagination::ChunkSize(JIRA_CHUNK_SIZE)))
}

// TODO: Consider adding progress bars here. Investigate these libraries:
// * https://crates.io/crates/progressing
// * https://crates.io/crates/linya
// * https://crates.io/crates/indicatif
/// Process the configured ticket queries into abstract tickets,
/// sorted in no particular order, which depends on the response from the issue tracker.
///
/// Downloads from Bugzilla and from Jira in parallel.
#[tokio::main]
pub async fn unsorted_tickets(
    queries: &[Arc<TicketQuery>],
    trackers: &tracker::Config,
) -> Result<Vec<AnnotatedTicket>> {
    // If no queries were found in the project configuration, quit with an error.
    // Such a situation should never occur because our config parsing requires at least
    // some items in the tickets file, but better make sure.
    if queries.is_empty() {
        bail!("No tickets are configured in this project.");
    }

    let queries: Vec<Arc<TicketQuery>> = queries.iter().map(Arc::clone).collect();

    let mut reference_queries: Vec<Arc<TicketQuery>> = Vec::new();

    // I don't know how to accomplish this in a functional style, unfortunately.
    for query in &queries {
        for reference in &query.references {
            reference_queries.push(Arc::clone(reference));
        }
    }

    // Download from Bugzilla and from Jira in parallel:
    let plain_bugs = bugs(&queries, trackers);
    let plain_issues = issues(&queries, trackers);
    let ref_bugs = bugs(&reference_queries, trackers);
    let ref_issues = issues(&reference_queries, trackers);

    // Wait until both downloads have finished:
    let (plain_bugs, plain_issues, ref_bugs, ref_issues) =
        tokio::try_join!(plain_bugs, plain_issues, ref_bugs, ref_issues)?;

    let mut references: HashMap<Arc<TicketQuery>, Vec<String>> = HashMap::new();
    for (query, bug) in ref_bugs {
        let ticket = bug.into_abstract(None, &trackers.bugzilla)?;
        references
            .entry(query)
            .and_modify(|e| e.push(ticket.format_signature()))
            .or_insert_with(|| vec![ticket.format_signature()]);
    }
    for (query, issue) in ref_issues {
        let ticket = issue.into_abstract(None, &trackers.jira)?;
        references
            .entry(query)
            .and_modify(|e| e.push(ticket.format_signature()))
            .or_insert_with(|| vec![ticket.format_signature()]);
    }
    eprintln!("These are the references: {:#?}", references);

    let mut results = Vec::new();

    // Convert bugs and issues into abstract tickets.
    // Using an imperative style so that each `into_abstract` call can return an error.
    for (query, bug) in plain_bugs {
        let attached_references = reattach_references(&query, &references);
        let ticket = bug.into_abstract(Some(attached_references), &trackers.bugzilla)?;
        let annotated = AnnotatedTicket { ticket, query };
        results.push(annotated);
    }
    for (query, issue) in plain_issues {
        let attached_references = reattach_references(&query, &references);
        let ticket = issue.into_abstract(Some(attached_references), &trackers.jira)?;
        let annotated = AnnotatedTicket { ticket, query };
        results.push(annotated);
    }

    // Modify each ticket by applying the overrides configured for it.
    for annotated_ticket in &mut results {
        annotated_ticket.override_fields();
    }

    Ok(results)
}

fn reattach_references(
    main_query: &Arc<TicketQuery>,
    references: &HashMap<Arc<TicketQuery>, Vec<String>>,
) -> Vec<String> {
    let needed_references = &main_query.references;
    references
        .iter()
        .filter(|(query, _references)| needed_references.contains(query))
        .flat_map(|(_query, references)| references)
        .cloned()
        .collect()
}

/// Extract queries of the `TicketQuery::Key` kind with their keys.
fn take_id_queries(queries: &[Arc<TicketQuery>]) -> Vec<(&str, Arc<TicketQuery>)> {
    queries
        .iter()
        .filter_map(|tq| {
            if let QueryUsing::Key(key) = &tq.using {
                Some((key.as_str(), Arc::clone(tq)))
            } else {
                None
            }
        })
        .collect()
}

/// Extract queries of the `TicketQuery::Search` kind with their searches.
fn take_search_queries(queries: &[Arc<TicketQuery>]) -> Vec<(&str, Arc<TicketQuery>)> {
    queries
        .iter()
        .filter_map(|tq| {
            if let QueryUsing::Search(search) = &tq.using {
                Some((search.as_str(), Arc::clone(tq)))
            } else {
                None
            }
        })
        .collect()
}

/// Download all configured bugs from Bugzilla.
/// Returns every bug in a tuple, annotated with the query that it came from.
async fn bugs(
    queries: &[Arc<TicketQuery>],
    trackers: &tracker::Config,
) -> Result<Vec<(Arc<TicketQuery>, Bug)>> {
    let bugzilla_queries: Vec<Arc<TicketQuery>> = queries
        .iter()
        .filter(|tq| tq.tracker == tracker::Service::Bugzilla)
        .map(Arc::clone)
        .collect();

    // If no tickets target Bugzilla, skip the download and return an empty vector.
    if bugzilla_queries.is_empty() {
        return Ok(Vec::new());
    }

    let queries_by_id = take_id_queries(&bugzilla_queries);
    let queries_by_search = take_search_queries(&bugzilla_queries);

    log::info!("Downloading bugs from Bugzilla.");
    let bz_instance = bz_instance(trackers)?;

    let mut all_bugs = Vec::new();

    let bugs_from_ids = bugs_from_ids(&queries_by_id, &bz_instance);
    let bugs_from_searches = bugs_from_searches(&queries_by_search, &bz_instance);

    let (mut bugs_from_ids, mut bugs_from_searches) =
        tokio::try_join!(bugs_from_ids, bugs_from_searches)?;

    all_bugs.append(&mut bugs_from_ids);
    all_bugs.append(&mut bugs_from_searches);

    log::info!("Finished downloading from Bugzilla.");

    Ok(all_bugs)
}

/// Download bugs that come from ID queries.
async fn bugs_from_ids(
    queries: &[(&str, Arc<TicketQuery>)],
    bz_instance: &bugzilla_query::BzInstance,
) -> Result<Vec<(Arc<TicketQuery>, Bug)>> {
    let bugs = bz_instance
        .bugs(
            &queries
                .iter()
                .map(|(key, _query)| *key)
                .collect::<Vec<&str>>(),
        )
        // This enables the download concurrency:
        .await
        .wrap_err("Failed to download tickets from Bugzilla.")?;

    let mut annotated_bugs: Vec<(Arc<TicketQuery>, Bug)> = Vec::new();

    for bug in bugs {
        let matching_query = queries
            .iter()
            .find(|(key, _query)| key == &bug.id.to_string().as_str())
            .map(|(_key, query)| Arc::clone(query))
            .ok_or_else(|| eyre!("Bug {} doesn't match any configured query.", bug.id))?;
        annotated_bugs.push((matching_query, bug));
    }

    Ok(annotated_bugs)
}

/// Download bugs that come from search queries.
async fn bugs_from_searches(
    queries: &[(&str, Arc<TicketQuery>)],
    bz_instance: &bugzilla_query::BzInstance,
) -> Result<Vec<(Arc<TicketQuery>, Bug)>> {
    let mut annotated_bugs: Vec<(Arc<TicketQuery>, Bug)> = Vec::new();

    for (search, query) in queries.iter() {
        let mut bugs = bz_instance
            .search(search)
            // This enables the download concurrency:
            .await
            .wrap_err("Failed to download tickets from Bugzilla.")?
            .into_iter()
            .map(|bug| (Arc::clone(query), bug))
            .collect();

        annotated_bugs.append(&mut bugs);
    }

    Ok(annotated_bugs)
}

/// Download all configured issues from Jira.
/// Returns every issue in a tuple, annotated with the query that it came from.
async fn issues(
    queries: &[Arc<TicketQuery>],
    trackers: &tracker::Config,
) -> Result<Vec<(Arc<TicketQuery>, Issue)>> {
    let jira_queries: Vec<Arc<TicketQuery>> = queries
        .iter()
        .filter(|&t| t.tracker == tracker::Service::Jira)
        .map(Arc::clone)
        .collect();

    // If no tickets target Jira, skip the download and return an empty vector.
    if jira_queries.is_empty() {
        return Ok(Vec::new());
    }

    let queries_by_id = take_id_queries(&jira_queries);
    let queries_by_search = take_search_queries(&jira_queries);

    log::info!("Downloading issues from Jira.");

    let jira_instance = jira_instance(trackers)?;

    let mut all_issues = Vec::new();

    let issues_from_ids = issues_from_ids(&queries_by_id, &jira_instance);
    let issues_from_searches = issues_from_searches(&queries_by_search, &jira_instance);

    let (mut issues_from_ids, mut issues_from_searches) =
        tokio::try_join!(issues_from_ids, issues_from_searches)?;

    all_issues.append(&mut issues_from_ids);
    all_issues.append(&mut issues_from_searches);

    log::info!("Finished downloading from Jira.");

    Ok(all_issues)
}

/// Download issues that come from ID queries.
async fn issues_from_ids(
    queries: &[(&str, Arc<TicketQuery>)],
    jira_instance: &jira_query::JiraInstance,
) -> Result<Vec<(Arc<TicketQuery>, Issue)>> {
    let issues = jira_instance
        .issues(
            &queries
                .iter()
                .map(|(key, _query)| *key)
                .collect::<Vec<&str>>(),
        )
        // This enables the download concurrency:
        .await
        .wrap_err("Failed to download tickets from Jira.")?;

    let mut annotated_issues: Vec<(Arc<TicketQuery>, Issue)> = Vec::new();

    for issue in issues {
        let matching_query = queries
            .iter()
            .find(|(key, _query)| key == &issue.key.as_str())
            .map(|(_key, query)| Arc::clone(query))
            .ok_or_else(|| eyre!("Issue {} doesn't match any configured query.", issue.id))?;
        annotated_issues.push((matching_query, issue));
    }

    Ok(annotated_issues)
}

/// Download issues that come from search queries.
async fn issues_from_searches(
    queries: &[(&str, Arc<TicketQuery>)],
    jira_instance: &jira_query::JiraInstance,
) -> Result<Vec<(Arc<TicketQuery>, Issue)>> {
    let mut annotated_issues: Vec<(Arc<TicketQuery>, Issue)> = Vec::new();

    for (search, query) in queries.iter() {
        let mut issues = jira_instance
            .search(search)
            // This enables the download concurrency:
            .await
            .wrap_err("Failed to download tickets from Bugzilla.")?
            .into_iter()
            .map(|issue| (Arc::clone(query), issue))
            .collect();

        annotated_issues.append(&mut issues);
    }

    Ok(annotated_issues)
}

// Temporarily disable this function while converting to configurable fields.
/*
/// Process a single ticket specified using the `ticket` subcommand.
#[tokio::main]
pub async fn ticket<'a>(
    id: &str,
    api_key: &str,
    service: Service,
    tracker: &'a tracker::Instance,
) -> Result<AbstractTicket<'a>> {
    match service {
        tracker::Service::Jira => {
            let jira_instance = jira_query::JiraInstance::at(host.to_string())?
                .authenticate(jira_query::Auth::ApiKey(api_key.to_string()))?;

            let issue = jira_instance.issue(id).await?;
            Ok(issue.into_abstract())
        }
        tracker::Service::Bugzilla => {
            let bz_instance = bugzilla_query::BzInstance::at(host.to_string())?
                .authenticate(bugzilla_query::Auth::ApiKey(api_key.to_string()))?
                .include_fields(BZ_INCLUDED_FIELDS.iter().map(ToString::to_string).collect());

            let bug = bz_instance.bug(id).await?;
            Ok(bug.into_abstract())
        }
    }
}
*/

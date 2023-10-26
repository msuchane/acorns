/*
acorns: Generate an AsciiDoc release notes document from tracking tickets.
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
use std::convert::From;
use std::sync::Arc;

use color_eyre::Result;

use crate::config::tracker;
use crate::config::TicketQuery;
use crate::ticket_abstraction::IntoAbstract;

/// A newtype that captures a list of ticket queries that are references,
/// formerly attached to actual release note ticket queries.
pub struct ReferenceQueries(pub Vec<Arc<TicketQuery>>);

impl From<&[Arc<TicketQuery>]> for ReferenceQueries {
    fn from(item: &[Arc<TicketQuery>]) -> Self {
        let mut reference_queries: Vec<Arc<TicketQuery>> = Vec::new();

        // I don't know how to accomplish this in a functional style, unfortunately.
        for query in item {
            for reference in &query.references {
                reference_queries.push(Arc::clone(reference));
            }
        }

        Self(reference_queries)
    }
}

/// String signatures of reference tickets, grouped by their ticket query.
/// An intermediate struct before attaching the signatures to release note tickets.
pub struct ReferenceSignatures(HashMap<Arc<TicketQuery>, Vec<String>>);

impl ReferenceSignatures {
    pub fn new<T: IntoAbstract, U: IntoAbstract>(
        ref_bugs: Vec<(Arc<TicketQuery>, T)>,
        ref_issues: Vec<(Arc<TicketQuery>, U)>,
        config: &tracker::Config,
    ) -> Result<Self> {
        let mut signatures: HashMap<Arc<TicketQuery>, Vec<String>> = HashMap::new();
        Self::store(&mut signatures, ref_bugs, &config)?;
        Self::store(&mut signatures, ref_issues, &config)?;

        // For each ticket, sort its references alphabetically.
        // Otherwise, the order changes based on the response from the ticket tracker,
        // which is random and produces distracting noise in output diffs.
        //
        // TODO: Is alphabetical sorting okay, or do we have to sort by the config file order instead?
        for references in signatures.values_mut() {
            references.sort_unstable();
        }

        Ok(Self(signatures))
    }

    /// A helper when building `ReferenceSignatures`. Abstracts over Bugzilla and Jira issues.
    /// Renders the signatures of the issues and records them in the shared `HashMap`.
    fn store<T: IntoAbstract>(
        signatures: &mut HashMap<Arc<TicketQuery>, Vec<String>>,
        ref_issues: Vec<(Arc<TicketQuery>, T)>,
        config: &tracker::Config,
    ) -> Result<()> {
        for (query, issue) in ref_issues {
            let ticket = issue.into_abstract(None, config)?;
            signatures
                .entry(query)
                .and_modify(|e| e.push(ticket.signature()))
                .or_insert_with(|| vec![ticket.signature()]);
        }

        Ok(())
    }

    /// Find references that belong to a ticket and return a list of them as signature strings.
    pub fn reattach_to(&self, main_query: &Arc<TicketQuery>) -> Vec<String> {
        let needed_references = &main_query.references;
        self.0
            .iter()
            .filter(|(query, _references)| needed_references.contains(query))
            .flat_map(|(_query, references)| references)
            .cloned()
            .collect()
    }
}

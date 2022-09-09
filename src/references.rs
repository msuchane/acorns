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
        Self::store(&mut signatures, ref_bugs, &config.bugzilla)?;
        Self::store(&mut signatures, ref_issues, &config.jira)?;

        Ok(Self(signatures))
    }

    /// A helper when building `ReferenceSignatures`. Abstracts over Bugzilla and Jira issues.
    /// Renders the signatures of the issues and records them in the shared `HashMap`.
    fn store<T: IntoAbstract>(
        signatures: &mut HashMap<Arc<TicketQuery>, Vec<String>>,
        ref_issues: Vec<(Arc<TicketQuery>, T)>,
        config: &tracker::Instance,
    ) -> Result<()> {
        for (query, issue) in ref_issues {
            let ticket = issue.into_abstract(None, config)?;
            signatures
                .entry(query)
                .and_modify(|e| e.push(ticket.format_signature()))
                .or_insert_with(|| vec![ticket.format_signature()]);
        }

        Ok(())
    }

    /// Find references that belong to a ticket a return a list of them as signature strings.
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

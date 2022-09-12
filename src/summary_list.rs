use std::collections::HashMap;
use std::fmt;

use askama::Template;
use color_eyre::{eyre::Context, Result};

use crate::extra_fields::DocTextStatus;
use crate::AbstractTicket;

// TODO: We might want these to be configurable.
const THROWAWAY_COMPONENTS: [&str; 3] = ["releng", "(none)", "Documentation"];
const THROWAWAY_PREFIXES: [&str; 2] = ["doc-", "Red_Hat_Enterprise_Linux-Release_Notes"];
const COMPONENT_PLACEHOLDER: &str = "other";

#[derive(Eq, PartialEq, PartialOrd, Ord)]
struct TicketsByComponent<'a> {
    component: PresentableComponent<'a>,
    signatures: Vec<String>,
}

/// All the data that the status table needs to render.
#[derive(Template)] // this will generate the code...
#[template(path = "summary-list.adoc", escape = "none")] // using the template in this path, relative
                                                         // to the `templates` dir in the crate root
struct SummaryList<'a> {
    tickets_by_components: &'a [TicketsByComponent<'a>],
}

#[derive(Eq, Hash, PartialEq, PartialOrd, Ord)]
enum PresentableComponent<'a> {
    Some(&'a str),
    None,
}

impl<'a> PresentableComponent<'a> {
    fn from(component: &'a str) -> Self {
        if THROWAWAY_COMPONENTS.contains(&component)
            || THROWAWAY_PREFIXES
                .iter()
                .any(|prefix| component.starts_with(prefix))
        {
            Self::None
        } else {
            Self::Some(component)
        }
    }
}

impl fmt::Display for PresentableComponent<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // If the variant is an actual component, format it with backticks as a code literal.
            PresentableComponent::Some(component) => write!(f, "`{}`", component),
            // If the variant is a throwaway component, replace it with an unformatted placeholder.
            PresentableComponent::None => write!(f, "{}", COMPONENT_PLACEHOLDER),
        }
    }
}

fn groups(tickets: &[AbstractTicket]) -> Vec<TicketsByComponent> {
    let mut components: HashMap<PresentableComponent, Vec<String>> = HashMap::new();

    tickets
        .iter()
        .filter(|ticket| ticket.doc_text_status == DocTextStatus::Approved)
        .for_each(|ticket| {
            for component in &ticket.components {
                let presentable = PresentableComponent::from(component);
                components
                    .entry(presentable)
                    .and_modify(|c| c.push(ticket.signature()))
                    .or_insert_with(|| vec![ticket.signature()]);
            }
        });

    components
        .into_iter()
        .map(|(component, signatures)| TicketsByComponent {
            component,
            signatures,
        })
        .collect()
}

pub fn appendix(tickets: &[AbstractTicket]) -> Result<String> {
    let mut groups = groups(tickets);

    // Sort the list by component name, alphabetically.
    // The 'other' group ends up at the very end, because it's a separate `enum` variant.
    groups.sort_unstable();

    let template = SummaryList {
        tickets_by_components: &groups,
    };

    template
        .render()
        .wrap_err("Failed to prepare the ticket appendix.")
}

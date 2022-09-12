use std::collections::HashMap;

use askama::Template;
use color_eyre::{eyre::Context, Result};

use crate::extra_fields::DocTextStatus;
use crate::AbstractTicket;

struct TicketsByComponent<'a> {
    component: &'a str,
    signatures: Vec<String>,
}

/// All the data that the status table needs to render.
#[derive(Template)] // this will generate the code...
#[template(path = "summary-list.adoc", escape = "none")] // using the template in this path, relative
                                                         // to the `templates` dir in the crate root
struct SummaryList<'a> {
    tickets_by_components: &'a [TicketsByComponent<'a>],
}

fn groups(tickets: &[AbstractTicket]) -> Vec<TicketsByComponent> {
    let mut components: HashMap<&str, Vec<String>> = HashMap::new();

    tickets
        .iter()
        .filter(|ticket| ticket.doc_text_status == DocTextStatus::Approved)
        .for_each(|ticket| {
            for component in &ticket.components {
                components
                    .entry(component)
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
    let groups = groups(tickets);
    let module = SummaryList {
        tickets_by_components: &groups,
    };

    module
        .render()
        .wrap_err("Failed to prepare the ticket appendix.")
}

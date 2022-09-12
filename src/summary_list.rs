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
use std::fmt;

use askama::Template;
use color_eyre::{eyre::Context, Result};

use crate::extra_fields::DocTextStatus;
use crate::templating::DocumentVariant;
use crate::AbstractTicket;

// TODO: We might want these to be configurable.
/// Documentation components that only categorize tickets internally.
const THROWAWAY_COMPONENTS: [&str; 3] = ["releng", "(none)", "Documentation"];
/// Prefixes shared by other internal, documentation components.
const THROWAWAY_PREFIXES: [&str; 2] = ["doc-", "Red_Hat_Enterprise_Linux-Release_Notes"];
/// The placeholder that renames the internal, documentation components.
const COMPONENT_PLACEHOLDER: &str = "other";

/// A list of all the ticket signatures that belong under this component.
#[derive(Eq, PartialEq, PartialOrd, Ord)]
struct TicketsByComponent<'a> {
    component: PresentableComponent<'a>,
    signatures: Vec<String>,
}

/// A representation of the AsciiDoc template for the appendix. Later rendered.
#[derive(Template)]
#[template(path = "summary-list.adoc", escape = "none")]
struct SummaryList<'a> {
    tickets_by_components: &'a [TicketsByComponent<'a>],
}

/// A wrapper around tickets components. It keeps all internal components separate
/// in the `Internal` variant. Public components are unchanged in the `Public` variant.
#[derive(Eq, Hash, PartialEq, PartialOrd, Ord)]
enum PresentableComponent<'a> {
    Public(&'a str),
    Internal,
}

impl<'a> PresentableComponent<'a> {
    /// Store the component either as public or as internal.
    fn from(component: &'a str) -> Self {
        if THROWAWAY_COMPONENTS.contains(&component)
            || THROWAWAY_PREFIXES
                .iter()
                .any(|prefix| component.starts_with(prefix))
        {
            Self::Internal
        } else {
            Self::Public(component)
        }
    }
}

impl fmt::Display for PresentableComponent<'_> {
    /// Display the component. Adds backticks for AsciiDoc formatting.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // If the variant is an actual component, format it with backticks as a code literal.
            PresentableComponent::Public(component) => write!(f, "`{}`", component),
            // If the variant is a throwaway component, replace it with an unformatted placeholder.
            PresentableComponent::Internal => write!(f, "{}", COMPONENT_PLACEHOLDER),
        }
    }
}

/// Group together all tickets by their component. Instead of full tickets, store just their signatures.
fn groups<'a>(
    tickets: &[&'a AbstractTicket],
    variant: DocumentVariant,
) -> Vec<TicketsByComponent<'a>> {
    // Use an intermediate `HashMap` for grouping.
    let mut components: HashMap<PresentableComponent, Vec<String>> = HashMap::new();

    tickets
        .iter()
        // Only include tickets with an approved doc text.
        // TODO: Include all tickets in the internal document variant.
        .filter(|ticket| filter_doc_text(ticket, variant))
        .for_each(|ticket| {
            for component in &ticket.components {
                let presentable = PresentableComponent::from(component);

                components
                    .entry(presentable)
                    .and_modify(|c| c.push(ticket.signature()))
                    .or_insert_with(|| vec![ticket.signature()]);
            }
        });

    // Convert the intermediate `HashMap` to the output `TicketsByComponent` format.
    components
        .into_iter()
        .map(|(component, signatures)| TicketsByComponent {
            component,
            signatures,
        })
        .collect()
}

/// A filter function that limits the tickets that are listed in the public document variant:
///
/// * In the public variant, only list tickets with an approved doc text.
/// * In the internal variant, list all tickets.
fn filter_doc_text(ticket: &AbstractTicket, variant: DocumentVariant) -> bool {
    match variant {
        DocumentVariant::Internal => true,
        DocumentVariant::Public => ticket.doc_text_status == DocTextStatus::Approved,
    }
}

/// Produce an AsciiDoc appendix file that lists all tickets in the document
/// by their component in a sorted table.
pub fn appendix(tickets: &[&AbstractTicket], variant: DocumentVariant) -> Result<String> {
    // Prepare ticket signatures grouped by component.
    let mut groups = groups(tickets, variant);

    // Sort the list by component name, alphabetically.
    // The 'other' group ends up at the very end, because it's a separate `enum` variant.
    groups.sort_unstable();

    // Pass the component groups to the AsciiDoc template.
    let template = SummaryList {
        tickets_by_components: &groups,
    };

    // Render the template as a valid AsciiDoc string.
    template
        .render()
        .wrap_err("Failed to prepare the ticket appendix.")
}

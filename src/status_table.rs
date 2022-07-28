use std::default::Default;

use askama::Template;

use crate::ticket_abstraction::AbstractTicket;

#[derive(Default)]
struct OverallProgress {
    all: u32,
    complete: u32,
    complete_pct: f32,
    warnings: u32,
    warnings_pct: f32,
    incomplete: u32,
    incomplete_pct: f32,
}

#[derive(Default)]
struct WriterStats<'a> {
    name: &'a str,
    total: u32,
    complete: u32,
    warnings: u32,
    incomplete: u32,
    percent: f32,
}

#[derive(Template)] // this will generate the code...
#[template(path = "status-table.html")] // using the template in this path, relative
                                 // to the `templates` dir in the crate root
struct StatusTableTemplate<'a> {
    products: &'a str,
    release: &'a str,
    overall_progress: OverallProgress,
    tickets: &'a [AbstractTicket],
    per_writer_stats: &'a [WriterStats<'a>],
    generated_date: &'a str,
}

fn main() {
    let status_table = StatusTableTemplate {
        products: "RHEL",
        release: "9.0",
        overall_progress: OverallProgress { ..Default::default() },
        per_writer_stats: &[],
        tickets: &[],
        generated_date: "",
    };
    println!("{}", status_table.render().unwrap());
}

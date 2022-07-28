use askama::Template; // bring trait in scope

#[derive(Template)] // this will generate the code...
#[template(path = "status-table.html")] // using the template in this path, relative
                                 // to the `templates` dir in the crate root
struct StatusTableTemplate<'a> {
    products: &'a str,
    release: &'a str,
}

fn main() {
    let status_table = StatusTableTemplate {
        products: "RHEL",
        release: "9.0",
    };
    println!("{}", status_table.render().unwrap());
}

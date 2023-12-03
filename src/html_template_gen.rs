use askama::Template; // bring trait in scope

#[derive(Template)] // this will generate the code...
#[template(path = "email_verification.html")] // using the template in this path, relative
                                              // to the `templates` dir in the crate root
pub struct VerifyEmailTemplate<'a> {
    name: &'a str,
    link: &'a str,
}

impl<'a> VerifyEmailTemplate<'a> {
    pub fn new(name: &'a str, reference: &'a str) -> Self {
        VerifyEmailTemplate {
            name,
            link: reference,
        }
    }
}

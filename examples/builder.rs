use chrono::{DateTime, Datelike, Utc};
use derive_builder::Builder;

#[allow(unused)]
#[derive(Builder, Debug)]
#[builder(build_fn(name = "_private_build"))]
struct User {
    #[builder(setter(into))]
    name: String,
    #[builder(setter(skip))]
    age: u8,
    #[builder(setter(into, strip_option), default)]
    email: Option<String>,
    #[builder(default = "vec![]", setter(each(name = "skill"), into))]
    skills: Vec<String>,
    #[builder(setter(custom))]
    dob: DateTime<Utc>,
}

fn main() -> anyhow::Result<()> {
    let user = User::build()
        .name("John")
        .dob("2000-01-01T00:00:00Z")
        .email("john@example.com")
        .skills(vec!["rust".to_string(), "python".to_string()])
        .build();
    println!("{:?}", user);
    Ok(())
}

impl User {
    pub fn build() -> UserBuilder {
        UserBuilder::default()
    }
}

impl UserBuilder {
    pub fn build(&self) -> anyhow::Result<User> {
        let mut user = self._private_build()?;
        user.age = (Utc::now().year() - user.dob.year()) as u8;
        Ok(user)
    }
    pub fn dob(&mut self, value: &str) -> &mut Self {
        self.dob = DateTime::parse_from_rfc3339(value)
            .map(|d| d.with_timezone(&Utc))
            .ok();
        self
    }
}

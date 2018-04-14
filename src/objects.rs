fn default_epoch() -> String {
    "1".to_owned()
}

#[derive(Deserialize, Debug)]
pub struct BuildSpec {
    pub name: String,
    pub version: String,
    #[serde(default = "default_epoch")]
    pub epoch: String,
    pub depends: Dependencies,
    pub sources: Vec<Source>,
    pub scripts: Scripts
}

fn empty_vec<T>() -> Vec<T> {
    vec![]
}

#[derive(Deserialize, Debug)]
pub struct Dependencies {
    #[serde(default = "empty_vec")]
    pub all: Vec<String>,
    #[serde(default = "empty_vec")]
    pub build: Vec<String>,
    #[serde(default = "empty_vec")]
    pub run: Vec<String>
}

#[derive(Deserialize, Debug)]
pub struct Source {
    pub href: String,
    pub verification: SourceVerification
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum SourceVerification {
    #[serde(rename = "sha256")]
    Sha256(Sum)
}

#[derive(Deserialize, Debug)]
pub struct Sum {
    pub sum: String
}

#[derive(Deserialize, Debug)]
pub struct Scripts {
    pub install: Vec<String>
}

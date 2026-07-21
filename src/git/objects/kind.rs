use anyhow::{Result, anyhow};

pub enum TreeEntryMode {
    RegularFile,
    ExecutableFile,
    Directory,
    Unsupported(String),
}

pub struct TreeEntry {
    pub mode: TreeEntryMode,
    pub name: String,
    pub sha1: Vec<u8>,
}

pub struct CommitPerson {
    pub name: String,
    pub email: String,
    pub timestamp: String,
    pub timezone: String,
}

pub enum ObjectKind {
    Blob(Vec<u8>),
    Tree(Vec<TreeEntry>),
    Commit {
        tree: String,
        parents: Vec<String>,
        author: CommitPerson,
        committer: CommitPerson,
        message: String,
    },
}

impl CommitPerson {
    pub fn demo() -> Self {
        CommitPerson {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            timestamp: "1234567890".to_string(),
            timezone: "+0000".to_string(),
        }
    }

    pub fn parse(input: &str) -> Result<Self> {
        // <name> <<email>> <timestamp> <timezone>
        // The name may contain spaces, so anchor on the email brackets rather
        // than splitting on whitespace from the front.
        let (name, rest) = input
            .split_once('<')
            .ok_or_else(|| anyhow!("expecting '<' before email"))?;
        let (email, rest) = rest
            .split_once('>')
            .ok_or_else(|| anyhow!("expecting '>' after email"))?;

        // `rest` is " <timestamp> <timezone>".
        let mut time_parts = rest.split_whitespace();
        let timestamp = time_parts
            .next()
            .ok_or_else(|| anyhow!("missing timestamp"))?;
        let timezone = time_parts
            .next()
            .ok_or_else(|| anyhow!("missing timezone"))?;

        return Ok(CommitPerson {
            name: name.trim().to_string(),
            email: email.to_string(),
            timestamp: timestamp.to_string(),
            timezone: timezone.to_string(),
        });
    }

    pub fn to_str(&self) -> String {
        format!(
            "{} <{}> {} {}",
            self.name, self.email, self.timestamp, self.timezone
        )
    }
}

impl TreeEntryMode {
    pub fn from_mode(mode: &str) -> Self {
        match mode {
            "100644" => TreeEntryMode::RegularFile,
            "100755" => TreeEntryMode::ExecutableFile,
            "40000" => TreeEntryMode::Directory,
            _ => TreeEntryMode::Unsupported(mode.to_string()),
        }
    }

    pub fn to_mode(&self) -> String {
        match self {
            TreeEntryMode::RegularFile => "100644".to_string(),
            TreeEntryMode::ExecutableFile => "100755".to_string(),
            TreeEntryMode::Directory => "40000".to_string(),
            TreeEntryMode::Unsupported(m) => m.clone(),
        }
    }
}

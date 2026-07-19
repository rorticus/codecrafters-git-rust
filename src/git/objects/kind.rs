use anyhow::{Result, anyhow, bail};

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
        parent: String,
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
        let parts: Vec<&str> = input.split(" ").collect();

        if parts.len() != 4 {
            bail!("malformed commita author");
        }

        return Ok(CommitPerson {
            name: parts[0].to_string(),
            email: parts[1]
                .strip_prefix("<")
                .ok_or_else(|| anyhow!("expecting email to start with <"))?
                .strip_suffix(">")
                .ok_or_else(|| anyhow!("expecting email to end with >"))?
                .to_string(),
            timestamp: parts[2].to_string(),
            timezone: parts[3].to_string(),
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

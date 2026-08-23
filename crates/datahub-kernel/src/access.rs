use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectRole {
    Viewer,
    Editor,
    Approver,
    Admin,
}

impl ProjectRole {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Viewer => "viewer",
            Self::Editor => "editor",
            Self::Approver => "approver",
            Self::Admin => "admin",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "viewer" => Some(Self::Viewer),
            "editor" => Some(Self::Editor),
            "approver" => Some(Self::Approver),
            "admin" => Some(Self::Admin),
            _ => None,
        }
    }

    #[must_use]
    pub const fn allows(self, action: ProjectAction) -> bool {
        match (self, action) {
            (_, ProjectAction::Read)
            | (Self::Editor | Self::Approver | Self::Admin, ProjectAction::Write)
            | (Self::Approver | Self::Admin, ProjectAction::Approve)
            | (Self::Admin, ProjectAction::ManageMembers) => true,
            (Self::Viewer, _)
            | (Self::Editor, ProjectAction::Approve | ProjectAction::ManageMembers)
            | (Self::Approver, ProjectAction::ManageMembers) => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectAction {
    Read,
    Write,
    Approve,
    ManageMembers,
}

#[cfg(test)]
mod tests {
    use super::{ProjectAction, ProjectRole};

    #[test]
    fn role_permissions_follow_least_privilege() {
        assert!(ProjectRole::Viewer.allows(ProjectAction::Read));
        assert!(!ProjectRole::Viewer.allows(ProjectAction::Write));
        assert!(ProjectRole::Editor.allows(ProjectAction::Write));
        assert!(!ProjectRole::Editor.allows(ProjectAction::Approve));
        assert!(ProjectRole::Approver.allows(ProjectAction::Approve));
        assert!(ProjectRole::Admin.allows(ProjectAction::ManageMembers));
    }
}

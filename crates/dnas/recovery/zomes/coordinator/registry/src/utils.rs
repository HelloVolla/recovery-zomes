use hdk::prelude::*;
use registry_integrity::*;
use shared_types::MemberStatus;

/// Create a link tag from MemberStatus for filtering
/// Tag format: 
/// - "invited:<base64_invite_code>" for Invited status
/// - "active" for Active status
/// - "revoked" for Revoked status
/// - "rejected:<reason>" for Rejected status
pub fn create_member_status_tag(status: &MemberStatus) -> LinkTag {
    let tag_string = match status {
        MemberStatus::Invited(invite_code) => format!("invited:{}", invite_code),
        MemberStatus::Active => "active".to_string(),
        MemberStatus::Revoked => "revoked".to_string(),
        MemberStatus::Rejected(reason) => format!("rejected:{:?}", reason),
    };
    LinkTag::new(tag_string.as_bytes().to_vec())
}

/// Helper function to update member links when status changes
pub fn update_member_links(
    base: impl Into<AnyLinkableHash>,
    old_target: ActionHash,
    new_target: ActionHash,
    new_status: &MemberStatus,
    link_type: LinkTypes,
) -> ExternResult<()> {
    let base_hash = base.into();
    
    // Delete old link
    let old_links = get_links(
        LinkQuery {
            base: base_hash.clone(),
            link_type: link_type.try_into_filter()?,
            tag_prefix: None,
            before: None,
            after: None,
            author: None,
        },
        GetStrategy::Local,
    )?;

    for link in old_links {
        if link.target == old_target.clone().into() {
            delete_link(link.create_link_hash, GetOptions::local())?;
        }
    }

    // Create new link with updated tag
    let new_tag = create_member_status_tag(new_status);
    create_link(
        base_hash,
        new_target,
        link_type,
        new_tag,
    )?;

    Ok(())
}

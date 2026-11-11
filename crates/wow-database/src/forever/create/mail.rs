//! Player::_SaveMail projection at 02245dcd, without canonical object mutation.
use super::{PreparedStatement, SqlTransaction, items};
use wow_persistence::forever::creation::save::{NewMailSave, NewMailSaveEntry};

#[cfg(test)]
#[path = "../../../unit_tests/forever/creation_mail.rs"]
mod tests;

const UPDATE: &str = "UPDATE mail SET has_items = ?, expire_time = ?, deliver_time = ?, money = ?, cod = ?, checked = ? WHERE id = ?";
const LINK_DELETE: &str = "DELETE FROM mail_items WHERE item_guid = ?";
const MAIL_DELETE: &str = "DELETE FROM mail WHERE id = ?";
const LINKS_DELETE: &str = "DELETE FROM mail_items WHERE mail_id = ?";

pub(super) fn append(transaction: &mut SqlTransaction, data: &NewMailSave) {
    if !data.updated {
        return;
    }
    for message in &data.messages {
        match message {
            NewMailSaveEntry::Unchanged => {}
            NewMailSaveEntry::Changed { removed_items, .. } => {
                transaction.append(changed_statement(message).expect("matched Changed"));
                for item in removed_items {
                    transaction.append(delete_statement(LINK_DELETE, *item));
                }
            }
            NewMailSaveEntry::Deleted {
                message_id,
                items: attachments,
            } => {
                for item in attachments {
                    items::append_mail_attachment_delete(transaction, *item);
                }
                transaction.append(delete_statement(MAIL_DELETE, *message_id));
                transaction.append(delete_statement(LINKS_DELETE, *message_id));
            }
        }
    }
}

fn changed_statement(message: &NewMailSaveEntry) -> Option<PreparedStatement> {
    let NewMailSaveEntry::Changed {
        message_id,
        items,
        expire_time,
        deliver_time,
        money,
        cod,
        checked,
        ..
    } = message
    else {
        return None;
    };
    let mut row = PreparedStatement::new(UPDATE);
    row.set_u8(0, u8::from(!items.is_empty()));
    row.set_i64(1, *expire_time);
    row.set_i64(2, *deliver_time);
    row.set_u64(3, *money);
    row.set_u64(4, *cod);
    row.set_u32(5, u32::from(*checked as u8));
    row.set_u64(6, *message_id);
    Some(row)
}

fn delete_statement(sql: &'static str, key: u64) -> PreparedStatement {
    let mut row = PreparedStatement::new(sql);
    row.set_u64(0, key);
    row
}

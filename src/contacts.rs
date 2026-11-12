use crate::crypto::validate_public_key;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Contact {
    pub name: String,
    pub public_key: String,
    pub fingerprint: String,
    pub created_at: String,
}

pub fn contacts_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|p| p.join("waxseal").join("contacts.json"))
}

fn legacy_contacts_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|p| p.join("mixin").join("contacts.json"))
}

fn load_contacts_from(primary: Option<&PathBuf>, legacy: Option<&PathBuf>) -> Vec<Contact> {
    let Some(path) = primary else {
        return Vec::new();
    };
    if path.exists() {
        return fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
    }
    if let Some(old_path) = legacy.filter(|p| p.exists()) {
        if let Ok(text) = fs::read_to_string(old_path) {
            if let Ok(contacts) = serde_json::from_str::<Vec<Contact>>(&text) {
                if let Some(parent) = path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let _ = fs::write(path, &text);
                return contacts;
            }
        }
    }
    Vec::new()
}

pub fn load_contacts() -> Vec<Contact> {
    load_contacts_from(contacts_path().as_ref(), legacy_contacts_path().as_ref())
}

pub fn save_contacts(contacts: &[Contact]) {
    if let Some(path) = contacts_path() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(contacts) {
            let _ = fs::write(path, json);
        }
    }
}

pub fn add_contact(
    contacts: &mut Vec<Contact>,
    name: &str,
    public_key: &str,
) -> Result<Contact, String> {
    let clean_name = name.trim();
    if clean_name.is_empty() {
        return Err("联系人姓名不能为空。".to_string());
    }
    let fingerprint = validate_public_key(public_key)?;
    let contact = Contact {
        name: clean_name.to_string(),
        public_key: public_key.trim().to_string(),
        fingerprint,
        created_at: current_date_str(),
    };
    contacts.push(contact.clone());
    save_contacts(contacts);
    Ok(contact)
}

pub fn remove_contact(contacts: &mut Vec<Contact>, index: usize) {
    if index < contacts.len() {
        contacts.remove(index);
        save_contacts(contacts);
    }
}

pub fn update_contact_name(
    contacts: &mut [Contact],
    index: usize,
    new_name: &str,
) -> Result<(), String> {
    let clean_name = new_name.trim();
    if clean_name.is_empty() {
        return Err("联系人姓名不能为空。".to_string());
    }
    if let Some(contact) = contacts.get_mut(index) {
        contact.name = clean_name.to_string();
        save_contacts(contacts);
        Ok(())
    } else {
        Err("找不到指定联系人。".to_string())
    }
}

fn current_date_str() -> String {
    // Simple date stamp without pulling in chrono dependency
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Approximate days since epoch: 86400 secs per day
    let days = secs / 86400;
    // Year approximation:
    let approx_year = 1970 + days / 365;
    let day_of_year = days % 365;
    let approx_month = (day_of_year / 30) + 1;
    let approx_day = (day_of_year % 30) + 1;
    format!("{approx_year:04}-{approx_month:02}-{approx_day:02}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{compute_fingerprint, generate_identity};

    #[test]
    fn test_contact_validation() {
        let mut list = Vec::new();
        assert!(add_contact(&mut list, "", "bad key").is_err());
        assert!(add_contact(&mut list, "Alice", "bad key").is_err());

        let (_priv, pub_key) = generate_identity().unwrap();
        let res = add_contact(&mut list, "Alice", &pub_key);
        assert!(res.is_ok());
        let c = res.unwrap();
        assert_eq!(c.name, "Alice");
        assert_eq!(c.fingerprint, compute_fingerprint(&pub_key));
        assert_eq!(list.len(), 1);

        remove_contact(&mut list, 0);
        assert_eq!(list.len(), 0);
    }

    #[test]
    fn test_contact_update_name() {
        let mut list = Vec::new();
        let (_priv, pub_key) = generate_identity().unwrap();
        add_contact(&mut list, "Alice", &pub_key).unwrap();
        assert_eq!(list[0].name, "Alice");

        assert!(update_contact_name(&mut list, 0, "   ").is_err());
        assert!(update_contact_name(&mut list, 1, "Alice New").is_err());

        assert!(update_contact_name(&mut list, 0, "Alice Work").is_ok());
        assert_eq!(list[0].name, "Alice Work");
    }

    #[test]
    fn test_legacy_contacts_migration() {
        let base = std::env::temp_dir().join(format!("waxseal_contacts_mig_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let legacy = base.join("mixin").join("contacts.json");
        let primary = base.join("waxseal").join("contacts.json");
        fs::create_dir_all(legacy.parent().unwrap()).unwrap();

        let (_priv, pub_key) = generate_identity().unwrap();
        let mut seed_list = Vec::new();
        let contact = Contact {
            name: "LegacyAlice".into(),
            public_key: pub_key.clone(),
            fingerprint: compute_fingerprint(&pub_key),
            created_at: "2025-01-01".into(),
        };
        seed_list.push(contact.clone());
        fs::write(&legacy, serde_json::to_string_pretty(&seed_list).unwrap()).unwrap();

        let loaded = load_contacts_from(Some(&primary), Some(&legacy));
        assert_eq!(loaded, seed_list);
        assert!(primary.exists(), "Contacts should be migrated to new waxseal path");

        let _ = fs::remove_dir_all(&base);
    }
}

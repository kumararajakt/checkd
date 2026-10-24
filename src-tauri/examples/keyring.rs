use checkd_lib::sync::auth::{KeyringStore, SecretStore};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = KeyringStore::new("checkd");
    let name = "keyring-check";

    match store.set(name, "hello") {
        Ok(()) => println!("the keyring accepted a write"),
        Err(err) => {
            eprintln!("the keyring is not usable: {err}");
            return Ok(());
        }
    }

    match store.get(name)? {
        Some(value) => println!("the keyring read back: {value}"),
        None => println!("the keyring lost the entry"),
    }

    store.delete(name)?;
    println!("the entry is deleted again");

    Ok(())
}

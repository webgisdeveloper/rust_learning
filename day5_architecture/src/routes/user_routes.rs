// Use super::super to navigate backup to the root
use crate::models::UserProfile;

pub fn mock_create_user_handleer() {
    // note salary is a private field, new method is still working in this case
    let new_user = UserProfile::new(
        String::from("John M"),
        String::from("Admin"),
        95_000.0,
    );
    println!("📡 [Actix Mock Route] Successfully processed payload for: {:?}", new_user);
}

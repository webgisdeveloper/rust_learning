// Use super::super to navigate backup to the root
use crate::models::UserProfile;

pub fn mock_create_user_handleer() {
    let new_user = UserProfile {
	username: String::from("John M"),
	role: String::from("Admin"),
    };
    println!("📡 [Actix Mock Route] Successfully processed payload for: {:?}", new_user);
}

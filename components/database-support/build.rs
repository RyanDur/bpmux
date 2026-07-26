fn main() {
    println!("cargo:rerun-if-changed=../../databases/conversation-db/migrations");
}

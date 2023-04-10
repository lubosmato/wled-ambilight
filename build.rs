use windres::Build;

fn main() {
    Build::new().compile("resources.rc").unwrap();
}

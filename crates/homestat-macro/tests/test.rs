use homestat_macro::include_cyw_regions;

#[test]
fn test_regions() {
    let regions = include_cyw_regions!("crates/homestat-macro/tests/flash-metadata.json");

    println!("regions: {:?}", regions);
}

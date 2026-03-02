fn main() {
    let domain = "www.example.co.uk";
    if let Ok(parsed) = addr::parse_domain_name(domain) {
        let root = parsed.root();
        let prefix = parsed.prefix();
        println!("Root: {:?}", root);
        println!("Prefix: {:?}", prefix);
    }
}

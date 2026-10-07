fn main() {
    println!("cargo::rustc-check-cfg=cfg(widget_config)");
    println!("cargo::rustc-cfg=widget_config");
}

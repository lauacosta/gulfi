use std::{fs, path::Path};

use lightningcss::{
    printer::PrinterOptions,
    stylesheet::{MinifyOptions, ParserOptions, StyleSheet},
};
use minify_js::{Session, TopLevelMode};

fn main() {
    println!("cargo::rerun-if-changed=assets/main.js");
    println!("cargo::rerun-if-changed=assets/style.css");

    if !Path::new("dist").exists() {
        fs::create_dir_all("dist").expect("Error al generar el directorio 'dist'");
    }

    let css_content =
        fs::read_to_string("assets/styles.css").expect("No se encontró 'assets/styles.css'");
    let mut stylesheet = StyleSheet::parse(&css_content, ParserOptions::default()).unwrap();
    stylesheet.minify(MinifyOptions::default()).unwrap();
    let res = stylesheet.to_css(PrinterOptions::default()).unwrap();
    fs::write("dist/styles.min.css", res.code).expect("No se ha podido escribir styles.min.css");

    let js_content = fs::read_to_string("assets/main.js").expect("No se encontró 'assets/main.js'");
    let session = Session::new();
    let mut out = Vec::new();
    minify_js::minify(
        &session,
        TopLevelMode::Global,
        js_content.as_bytes(),
        &mut out,
    )
    .expect("No se ha podido minimizar el codigo JS.");
    fs::write("dist/main.min.js", out).expect("No se ha podido escribir main.min.js");
}

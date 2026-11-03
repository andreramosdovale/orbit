/// Regenera `src/bindings.ts`. A CI roda os testes e falha se o arquivo mudar.
#[test]
fn export_bindings() {
    moon_lib::export_bindings();
}

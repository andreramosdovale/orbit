# Como publicar uma versão

Publicar é grátis e não precisa de chave. O GitHub gera o instalador sozinho, e os amigos veem o aviso de atualização na próxima vez que abrirem o Orbit.

## Passos

1. Mude `version` em `[workspace.package]` no `Cargo.toml` da raiz. Exemplo: `0.1.1`.
2. Rode `cargo check -p moon` para atualizar o `Cargo.lock`, e leve os dois arquivos até a `main` pelo fluxo normal, com a CI verde.
3. Na `main` atualizada, crie e envie a tag, com um `v` na frente da mesma versão:

   ```sh
   git tag v0.1.1
   git push origin v0.1.1
   ```

4. Acompanhe em **Actions > Release** no GitHub. A primeira compilação leva bem mais tempo que as seguintes, porque baixa o WebRTC pré-compilado.

Se a tag for diferente da versão do `Cargo.toml`, o workflow falha sem publicar nada. Para tentar de novo, apague a tag e crie outra:

```sh
git push origin :refs/tags/v0.1.1
git tag -d v0.1.1
```

## Se uma versão sair com defeito

O aviso aparece para todos assim que o release é publicado. Corrija e publique uma versão maior: o app só oferece versões mais novas que a instalada. Apagar o release com defeito no GitHub também faz o aviso sumir para quem ainda não atualizou.

## Gerar o instalador no próprio PC

```sh
cd moon && npm run tauri build
```

O arquivo sai em `target/release/bundle/nsis/Orbit_<versão>_x64-setup.exe`.

## Segurança

A conta do GitHub é o que protege as atualizações ([ADR 0019](decisions/0019-distribuicao-por-github-releases-sem-chaves.md)). Mantenha a autenticação em dois fatores ligada.

# Baseus Desktop — alterações de diagnóstico

Esta cópia contém modificações locais sobre o código da branch main (base 0.3.0).
Não é uma versão oficial publicada pelo autor e não inclui um executável compilado.

## O que mudou

- Reconhecimento dos nomes exatos Bass BP1 Pro, Baseus Bass BP1 Pro e suas variantes com ANC, sem diferenciar maiúsculas e minúsculas; espaços nas extremidades são ignorados na busca.
- Busca por UUID de serviço continua prioritária. Não aceita outros modelos apenas por conterem a palavra Baseus.
- Quando a busca termina sem correspondência, o erro informa quantos dispositivos BLE foram observados durante a tentativa. Não exibe nomes nem endereços de outros dispositivos.
- O último erro de conexão aparece na interface e permanece durante as novas tentativas. É removido quando a conexão funciona e pode ser recuperado se a janela abrir após o erro.
- Falhas ao consultar ou instalar atualizações aparecem na tela, com opção de tentar novamente.
- O painel de instalação continua visível enquanto a atualização está sendo instalada.
- Teste de reconhecimento adicionado e incluído no CI do Windows.

Não foi acrescentado um fallback RFCOMM: nos testes do usuário, o SPP é encontrado, mas a conexão retorna WSAEADDRINUSE (10048). A causa desse erro permanece não confirmada. Estas mudanças não prometem resolver a conexão física.

## Validação realizada

- TypeScript: `npx tsc --noEmit` aprovado.
- Interface: `npm run build` aprovado.
- Protocolo: `cargo test -p baseus-protocol`, 27 testes aprovados.
- Transporte e seu teste: `cargo check -p baseus-transport --tests --target x86_64-pc-windows-msvc` aprovado. Verifica compilação, não executa o teste Windows.
- A execução dos testes de transporte em Linux ficou bloqueada pela falta das bibliotecas de desenvolvimento D-Bus.
- O aplicativo Tauri completo e o instalador Windows não foram compilados neste ambiente. Não houve teste Bluetooth real desta cópia.

## Gerar um instalador no Windows

Pré-requisitos: Rust estável com alvo MSVC, Visual Studio Build Tools com desenvolvimento Desktop em C++, WebView2, Node.js e pnpm 9.
Na pasta raiz extraída:

```powershell
cargo test -p baseus-protocol -p baseus-transport
cd apps/baseus-app
pnpm install --frozen-lockfile
pnpm exec tsc --noEmit
pnpm tauri build --bundles nsis
```

O instalador será gerado em `target/release/bundle/nsis/`, na raiz do projeto.
Encerre o app original antes de testar. A cópia mantém o identificador original do app e pode compartilhar configurações ou substituir sua instalação. Guarde o instalador 0.2.1 para poder voltar.

## Alternativa: gerar pelo GitHub Actions

O arquivo `.github/workflows/diagnostic-build.yml` permite executar manualmente **Windows diagnostic build** em um repositório seu que contenha estas alterações. Ele verifica o código, roda os testes e produz um artefato com o instalador NSIS. Não publica uma release e não precisa da chave de assinatura do autor. O fluxo foi preparado, mas não executado aqui.

## Testes com o fone

1. Com o fone indisponível, aguarde uma busca completa (20 segundos mais inicialização). Confira a mensagem e a contagem de dispositivos.
2. Abra o estojo e verifique se conecta; se não, copie o erro exibido. A contagem BLE não representa apenas fones Baseus.
3. Ao conectar, confira que o erro desaparece e teste bateria e ANC.
4. Em Settings, clique em Check for updates. Confira que uma falha deixa mensagem visível. O atualizador continua apontando para as releases do autor original.

A documentação não contém o endereço Bluetooth, o nome do PC nem outros identificadores do usuário.

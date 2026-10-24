# Cenário: Operação Rota Silenciosa — tráfico de pessoas (100% sintético)

Cenário de treinamento/demonstração para investigação anti-tráfico de pessoas
(Protocolo de Palermo · Lei 13.344/2016). **Todos os nomes, números, URLs,
carteiras e locais são fictícios** (domínios `.example`, telefones reservados).

## Narrativa

| Papel | Entidade | Canal |
|---|---|---|
| Recrutador | `S-REC-01` (`vagasmodelosbr`, +55 85 98877-0404) | Instagram "vaga de modelo/hostess", grupo "Vagas Europa 2026" |
| Transportador | `S-TRA-02` (`douglasviagens`, van `BRA2E19`) | Fortaleza → São Paulo → Curitiba → Foz do Iguaçu (BR-277) |
| Controladora | `S-CTL-03` (`lua_sp2026`, `bella_ctba`, `nina_zonasul`) | Anúncios em `acompanhantesbr.example` / `classilove.example`, telefones rotativos |
| Financiador | `S-FIN-04` (`emservicos`, EM Serviços LTDA) | PIX, dinheiro, BTC/ETH, transferência antes da travessia |
| Vítimas | `V-01…V-06` (rótulos fingerprintados no grafo) | documentos retidos, "dívida" de R$ 8.000, sinais de menor idade |

Indicadores plantados nos dados: controle por terceiro (agência/assessor/recepção),
rotação de chip ("novo número, mesma agência"), telefone reutilizado em várias
cidades, hotel/apartamento compartilhado, liberdade restrita, dívida/meta,
documento retido, idade no limiar, travessia de fronteira planejada, CNPJ de
fachada e conversão dos proventos em cripto.

## Arquivos

- `21_human-trafficking_ads.csv` — 12 anúncios (URL, handle, telefone, site, cidade/geo, texto, categoria, idade declarada).
- `22_human-trafficking_network.csv` — 19 registros da cadeia recrutamento → transporte → exploração → finanças (suspeito, vítima, conta, telefone, placa, hotel, rota, status do documento).
- `23_human-trafficking_finance.csv` — 10 transações (PIX/dinheiro/BTC/ETH) ligando contas, carteiras e o CNPJ de fachada.
- `plugin-human-trafficking.json` — plugin de classificação: mapeamentos de colunas, 30+ sinais de risco e adendo de prompt (Palermo / centrado na vítima).

## Como rodar

```bash
# 1) instalar o plugin (Ajustes → Plugins → Importar manifesto, ou copiar):
cp scenarios/human-trafficking/plugin-human-trafficking.json ~/.cortexintel/plugins/

# 2) na GUI: Novo projeto → vertical "human-trafficking" → Executar análise
#    com os 3 CSVs (ou pela CLI):
cortex run --domain human-trafficking \
  scenarios/human-trafficking/21_human-trafficking_ads.csv \
  scenarios/human-trafficking/22_human-trafficking_network.csv \
  scenarios/human-trafficking/23_human-trafficking_finance.csv

# 3) Ajustes → Loja de Transforms → categoria "Anti-Tráfico de Pessoas" → instalar
```

## Transforms (categoria `trafficking`)

| id | entrada | o que faz | chave |
|---|---|---|---|
| `ht.ad-indicators` | url / report / communication / account | pontua indicadores lexicais pt/en do anúncio (terceiro, menor, movimento, liberdade restrita, dívida, dinheiro, rotação) → `incident` com score/band | não |
| `ht.phone-pivot` | selector / account | pivota o telefone/handle sobre o corpus local de anúncios (`params.corpus` ou `CORTEX_HT_ADS_CSV`) → anúncios, contas, cidades reutilizando o número | não |
| `ht.phone-lookup` | selector | operadora / país / tipo de linha (VoIP) via API numverify-compatível | `numverify` |
| `ht.handle-pivot` | account | sonda existência do handle em plataformas públicas (HEAD) | não |
| `ht.wallet-trace` | wallet | contrapartes BTC/ETH via Blockstream/Blockchair | não |
| `ht.route-timeline` | victim / suspect / person / device | reconstrói rota `A>B>C` → cadeia de locais `moved_to` (Rust) | não |
| `ht.site-image-match` | media / url | envia hash/imagem a serviço de identificação de quartos de hotel (`params.endpoint`, ex.: instância TraffickCam) | não |
| `ht.document-check` | victim / person / suspect / selector / organization | valida CPF/CNPJ/passaporte, sinaliza documento retido / inconsistente / menor declarado | não |
| `ht.referral-package` | qualquer | monta pacote de encaminhamento (Disque 100 / PF / NETP) com cadeia de custódia | não |

## Agentes (Biblioteca de agentes → categoria "Anti-Tráfico")

`Rede de Anúncios`, `Padrão de Recrutamento`, `Triagem de Segurança da Vítima`,
`Rastreio de Proventos`, `Rota & Movimento`, `Pacote de Evidências & Encaminhamento`.

## Perguntas para o copiloto (Ask AI)

- "Qual telefone aparece em mais cidades e quem controla as contas ligadas a ele?"
- "Quais vítimas têm documento retido e travessia de fronteira planejada? Priorize."
- "Reconstrua a rota da van BRA2E19 e o próximo destino provável."
- "Para onde vai o dinheiro das 'dívidas' e quais contas congelar?"

## Ética

Uso exclusivo por equipes com base legal (polícia, MP, hotlines, ONGs). Indicadores
são pistas a corroborar, nunca prova. Abordagem centrada na vítima: nunca contate a
vítima por canais controlados pelo explorador; encaminhe a Disque 100 / PF / NETP /
CREAS. Não exponha identidades além da necessidade operacional (LGPD).

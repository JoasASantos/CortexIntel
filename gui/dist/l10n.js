/* ===== CortexIntel — runtime localization layer =====
   app.js/ui.js render many labels in English. This layer translates rendered
   text nodes, tooltips, placeholders and titles on the fly for pt/es via an
   exact-match dictionary plus regex templates for dynamic strings. AI answers
   already arrive in the UI language (the backend enforces it), and technical
   identifiers are never touched. */
(function(){
"use strict";
const PT = {
  // nav / views
  "Dashboard":"Painel","Command Center":"Centro de Comando","Graph":"Grafo","Entities":"Entidades","Intelligence":"Inteligência","Priority":"Priorização","Agents":"Agentes","Sources":"Fontes","Timeline":"Timeline","Alerts":"Alertas","Reports":"Relatórios","Settings":"Ajustes","Entity Registry":"Registro de Entidades",
  "Switch project":"Trocar projeto","All projects…":"Todos os projetos…","Export project":"Exportar projeto","Import project":"Importar projeto","New project":"Novo projeto","Open graph":"Abrir grafo","Run analysis":"Executar análise","Run":"Executar","Ask AI":"Perguntar à IA","Ask AI (⌘/)":"Perguntar à IA (⌘/)","Run analysis (⌘R)":"Executar análise (⌘R)","Toggle theme":"Alternar tema","Notifications":"Notificações","Account":"Conta","Activity log (⌘D)":"Log de atividade (⌘D)",
  "Search entities, commands…":"Buscar entidades, comandos…","Search entities…  (⌘K commands)":"Buscar entidades… (⌘K comandos)","idle":"ocioso","Idle":"Ocioso","busy":"ocupado","running":"executando","complete":"concluído","failed":"falhou","transform":"transform","intel":"inteligência","pdf":"pdf",
  // dashboard
  "Decision readiness":"Prontidão para decisão","Data quality":"Qualidade dos dados","Avg confidence":"Confiança média","High-risk entities":"Entidades de alto risco","Unresolved":"Não resolvidas","Missing evidence":"Evidência faltante","Active hypotheses":"Hipóteses ativas","Graph coverage":"Cobertura do grafo",
  "ready":"pronto","needs review":"precisa de revisão","insufficient":"insuficiente","conflicting":"conflitante","clean":"limpo","fair":"razoável","incomplete":"incompleto","weak":"fraco","solid":"sólida","soft":"fraca","critical + high":"crítico + alto","low resolution conf.":"baixa confiança de resolução","AI-proposed, unconfirmed":"proposto pela IA, não confirmado",
  "Top priorities":"Principais prioridades","Risk overview":"Panorama de risco","Recommended next actions":"Próximas ações recomendadas","Recent signals":"Sinais recentes","Your projects":"Seus projetos","No projects yet.":"Nenhum projeto ainda.","No data yet — run an analysis or import a source.":"Sem dados ainda — execute uma análise ou importe uma fonte.",
  "Critical":"Crítico","High":"Alto","Medium":"Médio","Low":"Baixo","critical":"crítico","high":"alto","medium":"médio","low":"baixo","no source":"sem fonte","no metadata":"sem metadados","isolated":"isoladas","likely duplicates":"prováveis duplicatas","sensitive":"sensíveis","No Source":"Sem fonte","No Metadata":"Sem metadados","Isolated":"Isoladas","Likely Duplicates":"Prováveis duplicatas","Sensitive":"Sensíveis",
  "Low quality lowers Intelligence confidence — resolve in Entities.":"Baixa qualidade reduz a confiança da Inteligência — resolva em Entidades.","Highest risk — may require escalation or protective action.":"Risco máximo — pode exigir escalonamento ou ação protetiva.","Duplicates distort clusters and inflate risk.":"Duplicatas distorcem clusters e inflam o risco.","Sparse data lowers resolution confidence.":"Dados esparsos reduzem a confiança de resolução.","Generate action plan from intelligence":"Gerar plano de ação a partir da inteligência","Data supports a decision — synthesize the product.":"Os dados sustentam uma decisão — sintetize o produto.","Generate intelligence":"Gerar inteligência","Baseline assessment of the current graph.":"Avaliação base do grafo atual.","Generate intelligence product":"Gerar produto de inteligência","Export PDF report":"Exportar relatório PDF","Run an analysis or add entities to surface priorities.":"Execute uma análise ou adicione entidades para ver prioridades.",
  "Impact High":"Impacto alto","Impact Medium":"Impacto médio","Impact Low":"Impacto baixo","Conf High":"Conf. alta","Conf Medium":"Conf. média","Conf Low":"Conf. baixa","impact high":"impacto alto","impact medium":"impacto médio","conf high":"conf. alta","conf medium":"conf. média","do":"ir",
  // entities
  "Total":"Total","High risk":"Alto risco","Low confidence":"Baixa confiança","Likely duplicates":"Prováveis duplicatas","All":"Todos","Duplicates":"Duplicatas","No relations":"Sem relações","High degree":"Muito conectadas","Manual":"Manuais","Needs review":"Precisa de revisão","From answer":"Da resposta",
  "Entity":"Entidade","Type":"Tipo","Risk":"Risco","Confidence":"Confiança","Quality":"Qualidade","Conns":"Conexões","Status":"Status","Tags":"Tags","resolved":"resolvida","review":"revisar","manual":"manual","hypothesis":"hipótese","reviewed":"revisada","Resolved":"Resolvida","Reviewed":"Revisada","Hypothesis":"Hipótese",
  "Isolate in graph":"Isolar no grafo","Bulk tag":"Tag em lote","Mark reviewed":"Marcar revisado","Clear":"Limpar","Add entity":"Adicionar entidade","Search / intent — 'high risk low confidence', 'no source', 'duplicates'…":"Busca / intenção — 'alto risco baixa confiança', 'sem fonte', 'duplicatas'…","sorted by":"ordenado por","matching":"correspondentes","showing":"exibindo",
  // graph workspace
  "Entities panel":"Painel de entidades","Toggle entities panel (⌘B)":"Painel de entidades (⌘B)","Selection mode (hold S · drag to lasso)":"Modo seleção (segure S · arraste para laçar)","Add entity (A)":"Adicionar entidade (A)","Connect two nodes (C)":"Conectar dois nós (C)","Find path (P)":"Encontrar caminho (P)","Zoom in (+)":"Aproximar (+)","Zoom out (−)":"Afastar (−)","Fit to screen (F)":"Ajustar à tela (F)","Re-run layout (L)":"Refazer layout (L)","Filters":"Filtros","Display settings":"Configurações de exibição","Export (PNG / JSON)":"Exportar (PNG / JSON)","Reset view (Esc)":"Redefinir visão (Esc)","AI copilot (⌘/)":"Copiloto IA (⌘/)","Layout":"Layout","Cluster":"Cluster","Force":"Força","Concentric":"Concêntrico","Hierarchy":"Hierarquia","Circle":"Círculo","Grid":"Grade","Cluster: off":"Cluster: desligado","By type":"Por tipo","By component":"Por componente",
  "Map":"Mapa","Overview":"Visão geral","Neighborhood":"Vizinhança","Full":"Completo","Network":"Rede","Score":"Score",
  "Select all visible":"Selecionar todos visíveis","Filter by type":"Filtrar por tipo","Search…":"Buscar…","nodes":"nós","selected":"selecionadas","Ask AI about selection":"Perguntar à IA sobre a seleção","Remove selected":"Remover selecionadas","Run transform on selection":"Rodar transform na seleção","Clear selection":"Limpar seleção","Entity type":"Tipo de entidade","Label / value":"Rótulo / valor","name, email, IP, domain…":"nome, e-mail, IP, domínio…","Full form (media, files)":"Formulário completo (mídia, arquivos)",
  "No nodes match your search":"Nenhum nó corresponde à busca","No entities yet — add one or run an analysis":"Nenhuma entidade ainda — adicione uma ou execute uma análise","No graph yet":"Nenhum grafo ainda","Types":"Tipos","Click to toggle · Alt-click to solo":"Clique para alternar · Alt+clique para isolar","more":"mais",
  "Properties":"Propriedades","Relationships":"Relações","Neighbors":"Vizinhos","Endpoints":"Extremidades","Transforms":"Transforms","Comments":"Comentários","Add a comment…":"Adicionar um comentário…","Post":"Enviar","Run transform / enrich":"Rodar transform / enriquecer","Edit":"Editar","Remove node":"Remover nó","Close (Esc)":"Fechar (Esc)","Add tag":"Adicionar tag","Copy":"Copiar","Copied":"Copiado","no metadata":"sem metadados","no direct relations":"sem relações diretas","no neighbors":"sem vizinhos","none":"nenhuma","predicted":"prevista","relationship":"relação","confidence":"confiança","No transforms for this kind. Install from Settings → Transforms.":"Sem transforms para este tipo. Instale em Ajustes → Transforms.","No comments yet.":"Nenhum comentário ainda.","loading…":"carregando…","Adjust risk":"Ajustar risco",
  "Expand (AI)":"Expandir (IA)","Connect":"Conectar","Isolate":"Isolar","Create alert":"Criar alerta","Alert created":"Alerta criado","resolution":"resolução","quality":"qualidade","conns":"conexões","risk":"risco",
  "Console":"Console","Shortcuts":"Atalhos","Clear log":"Limpar log","Open / close":"Abrir / fechar","CortexIntel workspace ready":"Workspace CortexIntel pronto","Keyboard shortcuts":"Atalhos de teclado","Command palette":"Paleta de comandos","Select all nodes":"Selecionar todos os nós","Paste selectors → entities":"Colar seletores → entidades","Multi-select":"Seleção múltipla","Box select":"Seleção em caixa","Selection mode":"Modo seleção","Find path":"Encontrar caminho","Edit selected":"Editar selecionado","Hide selected":"Ocultar selecionados","Merge selected":"Mesclar selecionados","Fit":"Ajustar","Re-layout":"Refazer layout","Zoom":"Zoom","Clear / close":"Limpar / fechar","This help":"Esta ajuda","Close":"Fechar",
  "Export":"Exportar","PNG (current view)":"PNG (visão atual)","PNG (full graph, 2x)":"PNG (grafo inteiro, 2x)","JSON (nodes + edges)":"JSON (nós + arestas)","CSV (entities)":"CSV (entidades)","Nothing to export":"Nada para exportar",
  "Node style":"Estilo do nó","Filled":"Preenchido","Outlined":"Contornado","Node size":"Tamanho do nó","Size by connections":"Tamanho por conexões","Label size":"Tamanho do rótulo","Show labels":"Mostrar rótulos","Show icons":"Mostrar ícones","Link width":"Espessura das arestas","Color links by target type":"Colorir arestas pelo tipo do destino","Arrows":"Setas","Edge labels":"Rótulos das arestas","On hover / select":"Ao passar / selecionar","Always":"Sempre","Dotted background":"Fundo pontilhado","Minimap":"Minimapa","Legend":"Legenda","reset":"redefinir",
  // context menus
  "Open details":"Abrir detalhes","Edit entity…":"Editar entidade…","Expand via AI":"Expandir via IA","Select neighbors":"Selecionar vizinhos","Focus neighbors":"Focar vizinhos","Isolate neighborhood":"Isolar vizinhança","Neighborhood lens (2 hops)":"Lente de vizinhança (2 saltos)","Find path from here…":"Encontrar caminho a partir daqui…","Connect to another node…":"Conectar a outro nó…","Pin position":"Fixar posição","Unpin position":"Soltar posição","Hide node":"Ocultar nó","Copy label":"Copiar rótulo","Flag":"Marcador","red":"vermelho","orange":"laranja","yellow":"amarelo","green":"verde","blue":"azul","purple":"roxo","Clear flag":"Limpar marcador","AI Geolocation (Gemini)":"Geolocalização IA (Gemini)","Collapse this cluster":"Recolher este cluster","Merge selected…":"Mesclar selecionados…","Run transform on selection…":"Rodar transform na seleção…","Select neighbors too":"Selecionar vizinhos também","Edit label…":"Editar rótulo…","Select endpoints":"Selecionar extremidades","Reverse direction":"Inverter direção","Delete relationship":"Excluir relação","Add entity…":"Adicionar entidade…","Paste selectors":"Colar seletores","Select all":"Selecionar tudo","Show hidden nodes":"Mostrar nós ocultos","Fit to screen":"Ajustar à tela","Re-run layout":"Refazer layout","Export PNG":"Exportar PNG","Display settings…":"Configurações de exibição…","Focus cluster":"Focar cluster","Remove":"Remover","Cancel":"Cancelar","Merge":"Mesclar","Keep as primary":"Manter como principal","Merged":"Mesclado",
  // modals / toasts
  "Open a project first":"Abra um projeto primeiro","Open or create a project first":"Abra ou crie um projeto primeiro","Add or load entities first":"Adicione ou carregue entidades primeiro","Select a node, then click Path — or right-click a node → Find path from here":"Selecione um nó e clique em Caminho — ou botão direito no nó → Encontrar caminho","No path between these entities":"Não há caminho entre estas entidades","View reset":"Visão redefinida","Node removed":"Nó removido","Nodes connected":"Nós conectados","Connection removed":"Conexão removida","Relationship removed":"Relação removida","No match":"Sem correspondência","No selectors found in clipboard":"Nenhum seletor encontrado na área de transferência","No selectors in clipboard":"Sem seletores na área de transferência","Clipboard unavailable — use ⌘V":"Área de transferência indisponível — use ⌘V","No transforms installed for these types — Settings → Transforms":"Sem transforms instalados para estes tipos — Ajustes → Transforms","Select entities first":"Selecione entidades primeiro","Select an entity":"Selecione uma entidade","Tagged":"Tag aplicada","Entity added — run transforms to analyze":"Entidade adicionada — rode transforms para analisar","Manual entity added: ":"Entidade manual adicionada: ","Label or file required":"Rótulo ou arquivo obrigatório","Running pipeline…":"Executando pipeline…","Provide an input path":"Informe um caminho de entrada","Run an analysis first":"Execute uma análise primeiro","Open a project with a graph":"Abra um projeto com grafo","Generating intelligence…":"Gerando inteligência…","Intelligence product generated":"Produto de inteligência gerado","Generating a PDF report…":"Gerando relatório PDF…","PDF report created":"Relatório PDF criado","Project exported":"Projeto exportado","Project imported":"Projeto importado","Entity updated":"Entidade atualizada","Deleted":"Excluído","Save":"Salvar","Save changes":"Salvar alterações","Skip":"Pular","Add":"Adicionar","Apply":"Aplicar","Continue":"Continuar","Delete":"Excluir","Install":"Instalar","Send":"Enviar","Refresh":"Atualizar","Reset":"Redefinir","Path":"Caminho","Expand":"Expandir","Cluster expanded":"Cluster expandido","Risk adjusted":"Risco ajustado","Edge label (optional)":"Rótulo da conexão (opcional)","Edit connection":"Editar conexão","Delete connection":"Excluir conexão","Tag":"Tag","Bulk tag":"Tag em lote",
  "Add entity":"Adicionar entidade","Media file (image / video / audio) — uploaded for metadata & authenticity analysis":"Arquivo de mídia (imagem / vídeo / áudio) — enviado para análise de metadados e autenticidade","Browse…":"Procurar…","no file selected":"nenhum arquivo selecionado","Attributes (key: value per line)":"Atributos (chave: valor por linha)","Label / value":"Rótulo / valor",
  "Cannot open project: ":"Não foi possível abrir o projeto: ","Run failed: ":"Falha na execução: ","Import failed: ":"Falha na importação: ","Copy failed":"Falha ao copiar","Download failed: ":"Falha no download: ","Transform: ":"Transform: ","Running ":"Executando ","Copy text":"Copiar texto",
  "AI Copilot":"Copiloto IA","✦ AI Copilot":"✦ Copiloto IA","Ask about this graph… e.g. 'which accounts share infrastructure and why?'":"Pergunte sobre este grafo… ex.: 'quais contas compartilham infraestrutura e por quê?'","✦ thinking…":"✦ pensando…","(no answer)":"(sem resposta)","Ask CortexIntel anything — summarize, take me to…, run analysis, show high risk, export report…":"Pergunte qualquer coisa — resumir, me leve para…, executar análise, mostrar alto risco, exportar relatório…","I can summarize the investigation, jump to any area, or run an action. Try: “summarize this case”, “go to entities”, “generate intelligence”, “show high-risk in the graph”, “export a PDF report”.":"Posso resumir a investigação, ir para qualquer área ou executar uma ação. Tente: “resuma este caso”, “ir para entidades”, “gerar inteligência”, “mostrar alto risco no grafo”, “exportar relatório PDF”.","↵ send · Esc close · this assistant works across the whole app":"↵ enviar · Esc fechar · este assistente funciona em todo o app","↑↓ navigate · ↵ select · Esc close":"↑↓ navegar · ↵ selecionar · Esc fechar","Type a command…":"Digite um comando…",
  // settings
  "Configuration & integrations.":"Configuração e integrações.","AI provider":"Provedor de IA","LLM backends":"Backends LLM","Recheck":"Verificar novamente","Connect a source":"Conectar uma fonte","Generic connectors":"Conectores genéricos","Saved connectors (this project)":"Conectores salvos (este projeto)","Transform store":"Loja de transforms","Installed transforms":"Transforms instalados","API keys":"Chaves de API","Service":"Serviço","Key":"Chave","Save key":"Salvar chave","Classifier plugins":"Plugins de classificação","Manifest format":"Formato do manifesto","Active project":"Projeto ativo","Users & access":"Usuários e acesso","Security posture":"Postura de segurança","Language":"Idioma","Sign out":"Sair","Name":"Nome","Role":"Papel","Models & routing":"Modelos e roteamento","Model routing":"Roteamento de modelos","Models":"Modelos",
  // timeline / alerts / reports
  "Audit & discovery events.":"Eventos de auditoria e descoberta.","Flagged for human review.":"Sinalizados para revisão humana.","Generated reports":"Relatórios gerados","Current brief":"Resumo atual","Run an analysis to produce a report.":"Execute uma análise para produzir um relatório.","Generate PDF report":"Gerar relatório PDF","No reports generated yet — click \"Generate PDF\".":"Nenhum relatório gerado ainda — clique em \"Gerar PDF\".",
  // misc
  "Yes":"Sim","No":"Não","OK":"OK","Loading…":"Carregando…","Open":"Abrir","Delete project":"Excluir projeto","Delete project?":"Excluir projeto?","Rename":"Renomear","Analyzed by":"Analisado por","Why":"Por quê","Take this action":"Executar esta ação","Next":"Próximo","View in graph":"Ver no grafo","Recommended":"Recomendado","Feasible":"Viável","High risk":"Alto risco","Open Entities":"Abrir Entidades","Show in graph":"Ver no grafo","Open Timeline":"Abrir Timeline","Open Map":"Abrir Mapa","Open Intelligence":"Abrir Inteligência",
  "job queued":"job na fila","exported PNG":"PNG exportado","exported JSON":"JSON exportado","exported CSV":"CSV exportado",
};
const ES = {
  "Dashboard":"Panel","Command Center":"Centro de Mando","Graph":"Grafo","Entities":"Entidades","Intelligence":"Inteligencia","Priority":"Priorización","Agents":"Agentes","Sources":"Fuentes","Alerts":"Alertas","Reports":"Informes","Settings":"Ajustes","Entity Registry":"Registro de Entidades","Switch project":"Cambiar proyecto","New project":"Nuevo proyecto","Open graph":"Abrir grafo","Run analysis":"Ejecutar análisis","Run":"Ejecutar","Ask AI":"Preguntar a la IA","Toggle theme":"Cambiar tema","Notifications":"Notificaciones","Account":"Cuenta",
  "Decision readiness":"Preparación para decidir","Data quality":"Calidad de datos","Avg confidence":"Confianza media","High-risk entities":"Entidades de alto riesgo","Unresolved":"Sin resolver","Missing evidence":"Evidencia faltante","Active hypotheses":"Hipótesis activas","Graph coverage":"Cobertura del grafo","Top priorities":"Prioridades principales","Risk overview":"Panorama de riesgo","Recommended next actions":"Próximas acciones recomendadas","Recent signals":"Señales recientes","Your projects":"Tus proyectos",
  "Critical":"Crítico","High":"Alto","Medium":"Medio","Low":"Bajo","Total":"Total","High risk":"Alto riesgo","Low confidence":"Baja confianza","Likely duplicates":"Duplicados probables","All":"Todos","Duplicates":"Duplicados","No relations":"Sin relaciones","High degree":"Muy conectadas","Manual":"Manuales","Needs review":"Necesita revisión","Entity":"Entidad","Type":"Tipo","Risk":"Riesgo","Confidence":"Confianza","Quality":"Calidad","Conns":"Conexiones","Status":"Estado","Tags":"Etiquetas",
  "Properties":"Propiedades","Relationships":"Relaciones","Neighbors":"Vecinos","Comments":"Comentarios","Edit":"Editar","Remove node":"Eliminar nodo","Connect":"Conectar","Isolate":"Aislar","Create alert":"Crear alerta","Console":"Consola","Shortcuts":"Atajos","Export":"Exportar","Filters":"Filtros","Map":"Mapa","Overview":"Vista general","Neighborhood":"Vecindad","Full":"Completo","Network":"Red","Cancel":"Cancelar","Save":"Guardar","Close":"Cerrar","Add entity":"Añadir entidad","Copy":"Copiar","Copied":"Copiado","Merge":"Fusionar","Remove":"Eliminar","Clear":"Limpiar","Open a project first":"Abre un proyecto primero","Select all":"Seleccionar todo","Hide node":"Ocultar nodo","Find path":"Buscar ruta",
};
// dynamic templates: [regex, replacement] (pt only; es falls back to en)
const PT_RX = [
  [/^Review (\d+) critical (entities|entity)$/, "Revisar $1 entidade(s) crítica(s)"],
  [/^Resolve (\d+) likely duplicates$/, "Resolver $1 prováveis duplicatas"],
  [/^Enrich (\d+) entities missing metadata$/, "Enriquecer $1 entidades sem metadados"],
  [/^Inspect dense cluster around "(.+)"$/, "Inspecionar o cluster denso em torno de \"$1\""],
  [/^(\d+) connections — a structural hub worth examining\.$/, "$1 conexões — um hub estrutural que merece exame."],
  [/^▸ Review (\d+) low-confidence entities$/, "▸ Revisar $1 entidades de baixa confiança"],
  [/^▸ Merge (\d+) likely duplicates$/, "▸ Mesclar $1 prováveis duplicatas"],
  [/^▸ Trace source for (\d+) entities$/, "▸ Rastrear a fonte de $1 entidades"],
  [/^▸ Isolate (\d+) high-risk entities in graph$/, "▸ Isolar $1 entidades de alto risco no grafo"],
  [/^▸ Generate intelligence product$/, "▸ Gerar produto de inteligência"],
  [/^▸ Export PDF report$/, "▸ Exportar relatório PDF"],
  [/^(\d+) nodes · (\d+) edges$/, "$1 nós · $2 arestas"],
  [/^(\d+) shown · (\d+) entities · (\d+) edges$/, "$1 exibidos · $2 entidades · $3 arestas"],
  [/^(\d+) matching · showing (\d+)(.*)$/, "$1 correspondentes · exibindo $2$3"],
  [/^sorted by (\w+) (.)$/, "ordenado por $1 $2"],
  [/^(\d+) selected$/, "$1 selecionadas"],
  [/^(\d+) neighbor\(s\)$/, "$1 vizinho(s)"],
  [/^(\d+) links$/, "$1 conexões"],
  [/^(\d+) conns$/, "$1 conexões"],
  [/^resolution (\d+%)$/, "resolução $1"],
  [/^quality (\w+)$/, (m,q)=>"qualidade "+(PT[q]||q)],
  [/^(\d+%) · avg confidence (\d+%) · data quality (\d+%) · coverage (\d+%) · (\d+) source\(s\)\.$/, "$1 · confiança média $2 · qualidade dos dados $3 · cobertura $4 · $5 fonte(s)."],
  [/^(\d+) no-source · (\d+) no-meta$/, "$1 sem fonte · $2 sem metadados"],
  [/^(\d+) isolated$/, "$1 isoladas"],
  [/^Top risk contributors: (.*)$/, "Maiores contribuintes de risco: $1"],
  [/^Isolated (\d+) critical entities$/, "$1 entidades críticas isoladas"],
  [/^Isolated (\d+) in graph$/, "$1 isoladas no grafo"],
  [/^Isolated (.+)$/, "Isolado: $1"],
  [/^(\d+) marked reviewed$/, "$1 marcadas como revisadas"],
  [/^Added (\d+) from clipboard$/, "$1 adicionadas da área de transferência"],
  [/^Already exists: (.*)$/, "Já existe: $1"],
  [/^(\d+) removed$/, "$1 removidas"],
  [/^Remove (\d+) nodes\??$/, "Remover $1 nós?"],
  [/^Merge (\d+) nodes$/, "Mesclar $1 nós"],
  [/^Run on (\d+) selected$/, "Rodar em $1 selecionadas"],
  [/^Path found · (\d+) hops$/, "Caminho encontrado · $1 saltos"],
  [/^Connected · "(.*)"$/, "Conectado · \"$1\""],
  [/^Expand cluster \((\d+)\)$/, "Expandir cluster ($1)"],
  [/^Done — (\d+) entities, (\d+) relationships$/, "Concluído — $1 entidades, $2 relações"],
  [/^Analysis complete: (\d+) entities$/, "Análise concluída: $1 entidades"],
  [/^Analysis: (\d+) entities, (\d+) relationships \((.+)\)$/, "Análise: $1 entidades, $2 relações ($3)"],
  [/^Running (.+)…$/, "Executando $1…"],
  [/^\+(\d+) entities$/, "+$1 entidades"],
  [/^(\d+) entity\(ies\) added$/, "$1 entidade(s) adicionada(s)"],
  [/^Alert on (.*)$/, "Alerta em $1"],
  [/^Tagged (.*)$/, "Tag aplicada: $1"],
  [/^updated (\d{4}-\d{2}-\d{2})$/, "atualizado em $1"],
  [/^(\d+) activities$/, "$1 atividades"],
  [/^(\d+) sources$/, "$1 fontes"],
  [/^(\d+) acts$/, "$1 ativ."],
  [/^✓ analyzed$/, "✓ analisado"],
  [/^analyzed$/, "analisado"],
  [/^Add or load entities first$/, "Adicione ou carregue entidades primeiro"],
  [/^Expanded (\d+) member\(s\) matching "(.*)"$/, "$1 membro(s) expandido(s) correspondendo a \"$2\""],
  [/^Expanded top (\d+) of cluster$/, "Top $1 do cluster expandido"],
  [/^Type an address$/, "Digite um endereço"],
  [/^hidden: (.*)$/, "oculto: $1"],
  [/^layout: (.*)$/, "layout: $1"],
  [/^added (\w+) "(.*)"$/, "adicionado $1 \"$2\""],
  [/^removed (\d+) node\(s\)$/, "$1 nó(s) removido(s)"],
  [/^merged (\d+) into (.*)$/, "$1 mesclado(s) em $2"],
  [/^job:(\w+) → (.*)$/, "job:$1 → $2"],
  [/^job (\S+) queued$/, "job $1 na fila"],
  [/^(\w+) concluído · (.*)$/, "$1 concluído · $2"],
  [/^Job (\w+) completed/, "Job $1 concluído"],
  [/^ — all (\d+) entities$/, " — todas as $1 entidades"],
  [/^ — entities grouped by type · double-click a cluster to drill in$/, " — entidades agrupadas por tipo · duplo clique num cluster para detalhar"],
  [/^ — (\d+) high\/critical entities and their neighbors$/, " — $1 entidades altas/críticas e seus vizinhos"],
  [/^ — no high\/critical entities in this graph$/, " — nenhuma entidade alta/crítica neste grafo"],
  [/^ — (\d+) hop\(s\) around "(.*)" · right-click another node → Neighborhood$/, " — $1 salto(s) em torno de \"$2\" · botão direito noutro nó → Vizinhança"],
  [/^ — not enough timestamped entities \(need created_at\/timestamp attributes\)$/, " — entidades com data insuficientes (precisa de created_at/timestamp)"],
  [/^ — drag the slider to reveal entities up to a point in time \((\d+) timestamped\)$/, " — arraste para revelar entidades até um ponto no tempo ($1 com data)"],
  [/^Select an entity for Neighborhood$/, "Selecione uma entidade para Vizinhança"],
  [/^Cluster: off$/, "Cluster: desligado"],
  [/^up to (.*)$/, "até $1"],
  [/^(.*) → cutoff$/, "$1 → corte"],
];
const SKIP_TAGS = new Set(["SCRIPT","STYLE","PRE","CODE","TEXTAREA","INPUT","CANVAS","SVG","KBD"]);
const SKIP_SEL = "#cy, .code, .mono-block, .report-body, .ask-msg, .askbar-msg, .kv .v, .kv .k, .np-row .lbl, .ctx-name, .ent-ent .label, .tf-name, .brand-name, .crumb b, .proj-card .pc-name, .sit-title, .tl-title, .comment, .cmt-text, .rel .rn, .graph-tip .gt-l, .palette-item .hint, [data-nol10n]";
function dict(){ const l=(typeof LANG!=="undefined"?LANG:"en"); return l==="pt"?PT:l==="es"?ES:null; }
function tx(text){ const d=dict(); if(!d) return null; const raw=text; const t=raw.trim(); if(!t) return null;
  if(d[t]!=null){ return raw.replace(t, d[t]); }
  if(d===PT){ for(const [rx,rep] of PT_RX){ if(rx.test(t)){ return raw.replace(t, typeof rep==="function"? t.replace(rx,rep) : t.replace(rx,rep)); } } }
  return null; }
const done = new WeakMap(); // node -> last translated value
function skip(el){ for(let e=el; e && e.nodeType===1; e=e.parentElement){ if(SKIP_TAGS.has(e.tagName)) return true; if(e.matches && e.matches(SKIP_SEL)) return true; } return false; }
function walk(root){ const d=dict(); if(!d) return;
  const it=document.createTreeWalker(root, NodeFilter.SHOW_TEXT|NodeFilter.SHOW_ELEMENT, null);
  let n; const texts=[]; const els=[];
  while((n=it.nextNode())){ if(n.nodeType===3) texts.push(n); else els.push(n); }
  for(const t of texts){ if(!t.parentElement||skip(t.parentElement)) continue; const v=t.nodeValue; if(done.get(t)===v) continue; const r=tx(v); if(r!=null&&r!==v){ t.nodeValue=r; done.set(t,r); } else done.set(t,v); }
  for(const e of els){ if(e.closest && e.closest("#cy, .report-body, .ask-msg")) continue; for(const a of ["data-tip","placeholder","title"]){ const v=e.getAttribute(a); if(v==null) continue; const key="attr:"+a+":"+v; if(done.get(e)===key) continue; const r=tx(v); if(r!=null&&r!==v){ e.setAttribute(a,r); done.set(e,"attr:"+a+":"+r); } } } }
let pending=false;
function schedule(){ if(pending) return; pending=true; requestAnimationFrame(()=>{ pending=false; walk(document.body); }); }
const mo=new MutationObserver(muts=>{ for(const m of muts){ if(m.type==="characterData"){ const t=m.target; if(t.parentElement&&!skip(t.parentElement)){ const v=t.nodeValue; if(done.get(t)!==v){ const r=tx(v); if(r!=null&&r!==v){ t.nodeValue=r; done.set(t,r); continue; } done.set(t,v); } } }
  else if(m.type==="attributes"){ const e=m.target; const a=m.attributeName; const v=e.getAttribute(a); if(v==null) continue; const r=tx(v); if(r!=null&&r!==v){ const key="attr:"+a+":"+r; if(done.get(e)!==key){ e.setAttribute(a,r); done.set(e,key); } } }
  else schedule(); } });
function start(){ walk(document.body); mo.observe(document.body,{subtree:true,childList:true,characterData:true,attributes:true,attributeFilter:["data-tip","placeholder","title"]}); }
if(document.readyState==="loading") document.addEventListener("DOMContentLoaded",start); else start();
// re-walk when language changes
const _setLang=window.setLang; if(typeof _setLang==="function"){ window.setLang=function(l){ _setLang(l); done && setTimeout(()=>walk(document.body),0); }; }
window.L10N={ tx, walk, PT, ES, add:(k,v)=>{ PT[k]=v; } };
})();

# Summary eval: prompt v2, judge claude-sonnet

| model | contradicted | critical stated | facts stated | code checks | avg time |
|---|---|---|---|---|---|
| qwen3:4b | 2/50 | 19/23 | 32/50 | 35/40 | 328s |
| gemma3:4b | 4/50 | 14/23 | 22/50 | 36/40 | 92s |

## Failed critical facts and contradictions

- **qwen3:4b** · accelerated-dragon · `game1-technical-endgame-win` → contradicted: Summary says in the first game he "realized he lost", but the fact says he won it.
- **qwen3:4b** · accelerated-dragon · `game2-advantage-lost-then-won` → contradicted: Summary says the second game "ended in a draw", but the fact says the streamer won it.
- **qwen3:4b** · argentine-anarchism · `1880-1916-is-oligarchy-narrative` → missing: Summary never mentions the 1880-1916 conservative order or the oligarchy golden-age narrative.
- **qwen3:4b** · cross-entropy · `minimum-at-p-equals-q` → missing: Summary says nothing about the minimum at Q=P, entropy, or KL divergence.
- **qwen3:4b** · future-of-capitalism · `decoupling-would-come-too-late` → missing: No mention of absolute decoupling or its timeline.
- **qwen3:4b** · stonewall-e4-knight · `white-vs-black-stonewall-knight` → missing: The summary mentions a query about a knight on e4 in the Stonewall but never says whose Stonewall or whose knight, nor that it is a viewer's tweet about the Dutch with Black's knight.
- **gemma3:4b** · accelerated-dragon · `three-games-as-black` → contradicted: The summary describes a single game ('attempts to play the accelerated dragon opening against an opponent'), not three games.
- **gemma3:4b** · argentine-anarchism · `anarchist-golden-age-early-1900s` → contradicted: The summary places the anarchist golden age in 'entre 1880 y 1916' rather than the early 1900s after FORA's 1901 founding.
- **gemma3:4b** · argentine-anarchism · `1880-1916-is-oligarchy-narrative` → contradicted: The summary presents 1880-1916 as anarchism's 'Edad de Oro' instead of a narrative of the oligarchy that the video rejects.
- **gemma3:4b** · build-claude-code · `takeaway-demystify` → missing: Summary only says the project aims to 'desmitificar el uso de la IA' and practice harness engineering, without stating agents are just LLM wrappers, that products sell UI, or that it is for learning not production.
- **gemma3:4b** · claude-duet-review · `topic-claude-duet` → missing: The summary calls Claude Duet 'a new personal agent from Anthropic' but never says it is a phone app synced with Claude or aimed at small business owners; the main claim about its form is not stated.
- **gemma3:4b** · cross-entropy · `minimum-at-p-equals-q` → missing: KL divergence is mentioned as 'wastefulness of suboptimal codes', but the summary never says cross-entropy is minimized when Q equals P or that the minimum is the entropy.
- **gemma3:4b** · future-of-capitalism · `decoupling-would-come-too-late` → missing: Decoupling is mentioned only as a technological hope; the summary gives no timeline (about 100 years) or argument that it comes too late.
- **gemma3:4b** · quantum-ai · `advantage-only-on-artificial-problem` → missing: Summary does not mention the roulette example or that the proven advantage is on an artificial problem with no practical application.
- **gemma3:4b** · stonewall-e4-knight · `white-vs-black-stonewall-knight` → missing: The summary mentions 'the E4 knight in the Stonewall structure' but never says whose knight it is or that Black plays the Stonewall.
- **gemma3:4b** · stonewall-e4-knight · `dont-take-on-d5` → contradicted: The summary frames the problem as capturing the knight on D5 being a positional mistake, not White's earlier cxd5 or Bg5 as the real mistakes, so it does not state the fact's claim.

## Faithfulness: claims the video doesn't support

| model | major | minor | faithful summaries |
|---|---|---|---|
| qwen3:4b | 5 | 13 | 3/10 |
| gemma3:4b | 2 | 10 | 1/10 |

- **gemma3:4b** · accelerated-dragon · major wrong: "the creator ultimately settles for a draw, recognizing the opponent’s defensive capabilities and the difficulty in converting their positional advantage" → The creator did not settle: the game ended in a draw amid a time scramble (~5 seconds left, 56:22) and he says he was dominating and could have won (57:00-57:14), not crediting opponent's defense.
- **gemma3:4b** · accelerated-dragon · minor unsupported: "Despite some initial tactical opportunities" → The opening is discussed as positional/strategic (endgame, bishop pair, 0:50-2:15); no initial tactical opportunities are the point, and the missed chances mentioned come much later (e.g. 42:59).
- **qwen3:4b** · accelerated-dragon · major wrong: "resulting in one loss and two draws" → Game 1 is described as an easy win with the cheeky king doing all the work (12:00-16:14), game 2 ends with his pawns promoting (35:41-35:49), and only game 3 is called a draw (56:59).
- **qwen3:4b** · accelerated-dragon · major wrong: "later realized he lost (a "minus 4" result)" → He says 'I didn't understand this is minus4. I thought I was barely winning' (14:12-14:18) about an evaluation, and the game is then reviewed as a winning one, not a loss.
- **qwen3:4b** · accelerated-dragon · major wrong: "The second game ended in a draw after he made endgame mistakes" → In game 2 his opponent made the mistakes and his pawns promoted (28:20, 35:41-35:49); the speaker's mistake was the opening move (24:32-24:38), and the game was not a draw.
- **qwen3:4b** · accelerated-dragon · minor misleading: "while the third game (against the Alapin variation) also concluded in a draw after a tense pawn race" → The tense pawn race was game 2 (33:52); game 3 against the Alapin was a draw after a time scramble (54:34, 56:59).
- **qwen3:4b** · accelerated-dragon · major unsupported: "but precise endgame play is essential to avoid losses" → The video draws no such conclusion; he calls it instructive technical chess (16:26-16:38), and no game is described as lost.
- **gemma3:4b** · argentine-anarchism · minor misleading: "A pesar del fracaso de la huelga del centenario de la independencia y la posterior represión, la FORA logró difundir ampliamente las ideas libertarias" → The 1910 centenary strike failure and repression came after the growth and golden age (24:00-24:19 says it led to the anarchists' displacement, to be covered in the next episode), so the 'despite... managed to spread' ordering reverses the chronology.
- **gemma3:4b** · argentine-anarchism · minor misleading: "aunque finalmente se vio debilitada por conflictos internos y la competencia de otras corrientes obreras" → The video only hints at the weakening and defers it to the next episode (31:05-31:18), mentioning competition from other currents and the repression; the internal split is described as FORA's 9th congress dispute (13:19-13:46), not as the cause of a final decline.
- **gemma3:4b** · argentine-anarchism · minor misleading: "el video examina el auge y la caída de esta importante fase del anarquismo argentino" → The video covers the rise and golden age and only foreshadows the decline, which it says it will analyze in the next episode (24:19-24:21, 31:15-31:18).
- **qwen3:4b** · argentine-anarchism · minor unsupported: "la división interna de la FORA tras el noveno congreso (1909)" → The video gives no year for the ninth congress (captions omit it at 12:28); 1909 is only the founding year of the CORA (12:07) and of the Semana Roja.
- **qwen3:4b** · argentine-anarchism · minor misleading: "concluyendo su período de influencia" → The video says the setback produced a progressive displacement of anarchists in labor leadership, to be analyzed in the next episode (24:11-24:21), not an end of their influence.
- **gemma3:4b** · browser-tabs · minor misleading: "además de integrar funcionalidades como el uso de semantic release para automatizar las versiones de las librerías" → Semantic release is a release-automation practice the author set up in the repo (4:18-4:58), not a functionality of the CLI utility, and it concerns this repo's releases rather than 'las librerías'.
- **qwen3:4b** · browser-tabs · minor unsupported: "no genera complicaciones adicionales" → The video only says the goal is that it not increase Codely's baseline operating cost ([3:52]-[4:15]); it never concludes it causes no complications.
- **qwen3:4b** · browser-tabs · minor misleading: "Esta herramienta automatiza la organización de pestañas sin incrementar el costo operativo diario" → Not increasing operating cost is a stated aim ('lo que quiero es que...', [3:50]-[3:55]), not a demonstrated result presented as fact.
- **gemma3:4b** · build-claude-code · minor unsupported: "Explica detalladamente el funcionamiento del “arnés” de IA, incluyendo el bucle agéntico, el uso de herramientas para realizar acciones como editar archivos, y la implementación de memoria." → Memory is only listed as a concept in the intro (0:30) and a context-compaction field is glimpsed in the agent struct (7:19); its implementation is never explained.
- **qwen3:4b** · build-claude-code · minor unsupported: "recomendando aprender mediante experimentación gradual en lugar de herramientas complejas" → He recommends entering gradually by building things yourself because many resources are confusing (11:11-11:22), not as an alternative to complex tools.
- **qwen3:4b** · build-claude-code · minor unsupported: "destacando la importancia de la práctica en lugar de teoría" → He says the best way to learn is to roll up your sleeves and build, even copying existing products (11:36-11:43), but never contrasts practice with theory; he even recommends reading the lessons and articles (7:21-7:36).
- **gemma3:4b** · claude-duet-review · minor wrong: "inability to create a desktop app" → The video says Duet has no desktop app and doesn't show up in the desktop app (5:44-5:52, 10:30-10:40), not that it cannot create a desktop app.
- **qwen3:4b** · claude-duet-review · minor wrong: "Despite being 100 times more polished than early Claude Dots" → The video compares Duet to OpenAI's Dots ('OpenAI's Dots did so so so poorly', 1:44; '100 times more polished than Dots was when it came out', 15:20), not a 'Claude Dots'.
- **qwen3:4b** · claude-duet-review · minor unsupported: "so it won't become the creator's primary agent" → The creator says only 'I'm just not sure it's going to become part of mine' (15:47), not that it definitely won't.
- **gemma3:4b** · cross-entropy · minor unsupported: "ultimately leading to a model that’s optimized for compression and, by extension, intelligence" → The video only says the compressor construction and the 'compression is intelligence' assessment come in the next part (29:21-30:05); it does not conclude here that the model is optimized for intelligence.
- **qwen3:4b** · cross-entropy · minor misleading: "It concludes that this approach demonstrates "compression is intelligence" by training language models to be optimal compressors." → The video does not conclude this; it only sets up the link to compression and defers the explanation and the assessment of 'compression is intelligence' to the next part (29:51-30:05).
- **qwen3:4b** · extreme-places · minor unsupported: "Switzerland's 374,142 WWII bunkers" → The video never gives 374,142; it says 'hundreds of thousands of bunkers dug under Swiss homes' (38:27) and the WWII strategy was mountain fortifications (48:32-49:07), not all these bunkers.
- **qwen3:4b** · extreme-places · major unsupported: "The video concludes that Finland's historical resilience and preparedness—despite proximity to Russia—create a unique balance of security and well-being, contrasting sharply with the ongoing struggles of the other extreme locations." → The video is a compilation of five of the creator's favorite films (0:18, 2:58:25) and ends with Cyprus and a personal note on The Human Element (2:58:18-2:58:39); it draws no such conclusion about Finland versus the others.
- **qwen3:4b** · extreme-places · minor misleading: "The video explores five extreme places where human adaptation to harsh conditions defines survival" → The video presents five favorite field films (0:18) on varied themes (bunkers, Neom, a divided Cyprus), not a unifying theme of adaptation to harsh conditions for survival.
- **qwen3:4b** · extreme-places · minor wrong: "Saudi Arabia's $1 trillion Neom city project" → Video calls it 'trillion-dollar' (1:03:58) but states the budget ballooned from $500 billion to an estimated $1.5 trillion (1:11:27-1:11:31).
- **gemma3:4b** · future-of-capitalism · major misleading: "Se exploran alternativas como el “Green New Deal” y el decrecimiento" → The video presents the Green New Deal as part of the green-growth framework that does not break with growth logic (6:55, 9:15-9:19, 10:31), not as an alternative to it.
- **gemma3:4b** · quantum-ai · minor wrong: "la dificultad de procesar datos cuánticos" → The video says the difficulty is converting classical data into quantum data (19:00-19:23), while processing quantum data is described as especially promising and advantageous (19:26-20:21).
- **gemma3:4b** · stonewall-e4-knight · minor wrong: "avoiding the immediate temptation to capture the knight on D5" → The video says not to capture on D5 with the c4 pawn (trading the c-pawn for the e6 pawn), not to capture a knight on D5 (0:21-0:32, 26:00).

## Judge vs human verdicts: 4/6 agree

- qwen3:4b · build-claude-code · `build-your-own-harness`: human contradicted, judge stated
- gemma3:4b · claude-duet-review · `topic-claude-duet`: human stated, judge missing

## Code checks failed

- gemma3:4b · browser-tabs: direct_start
- qwen3:4b · browser-tabs: direct_start
- gemma3:4b · build-claude-code: direct_start
- qwen3:4b · cross-entropy: direct_start
- qwen3:4b · extreme-places: length_3_to_5_sentences, direct_start
- gemma3:4b · future-of-capitalism: direct_start
- gemma3:4b · quantum-ai: direct_start
- qwen3:4b · stonewall-e4-knight: direct_start

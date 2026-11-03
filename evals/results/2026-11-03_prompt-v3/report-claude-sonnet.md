# Summary eval: prompt v3, judge claude-sonnet

| model | contradicted | critical stated | facts stated | code checks | avg time |
|---|---|---|---|---|---|
| qwen3:4b | 2/50 | 19/23 | 33/50 | 37/40 | 287s |
| gemma3:4b | 3/50 | 12/23 | 20/50 | 35/40 | 87s |

## Failed critical facts and contradictions

- **qwen3:4b** · accelerated-dragon · `game1-technical-endgame-win` → contradicted: The summary says the first game was "a drawn position after careful maneuvering", but the fact says the streamer won it.
- **qwen3:4b** · accelerated-dragon · `game2-advantage-lost-then-won` → contradicted: The summary says the creator's mistake "led to a draw", but the fact says he still won after the opponent's mistakes.
- **qwen3:4b** · argentine-anarchism · `1880-1916-is-oligarchy-narrative` → missing: The summary never mentions the 1880-1916 conservative order or the oligarchy's golden-age narrative.
- **qwen3:4b** · cross-entropy · `minimum-at-p-equals-q` → missing: The summary says nothing about the minimum at P equals Q, the entropy, or KL divergence.
- **qwen3:4b** · cross-entropy · `llm-pretraining-loss` → missing: It only says it "explains how cross-entropy loss functions in pre-training", without stating the negative log probability of the true next token or why the log is forced.
- **qwen3:4b** · future-of-capitalism · `decoupling-would-come-too-late` → missing: The summary mentions reliance on decoupling but not that it might take about 100 years, too late for the 10-20 year window.
- **gemma3:4b** · accelerated-dragon · `three-games-as-black` → contradicted: The summary describes a single game ('In this game against an opponent playing the Alapin Sicilian'), but the video shows three games.
- **gemma3:4b** · argentine-anarchism · `anarchist-golden-age-early-1900s` → contradicted: The summary places anarchism's golden age across the whole 1880-1916 conservative period ("entre 1880 y 1916, el anarquismo experimentó su propia 'Edad de Oro'") instead of the early 1900s after FORA's 1901 founding.
- **gemma3:4b** · argentine-anarchism · `1880-1916-is-oligarchy-narrative` → missing: The summary never mentions the golden-age claim being an oligarchic narrative the video rejects, nor the argument that exclusion made workers receptive to anarchism.
- **gemma3:4b** · build-claude-code · `takeaway-demystify` → missing: Summary never states that agents are just wrappers around LLM calls, that products sell the UI, or that the harness is for learning not production.
- **gemma3:4b** · cross-entropy · `cross-entropy-definition` → missing: It calls cross-entropy 'a measure of the difference between two probability distributions' but does not give the bits-per-symbol definition with a code optimized for Q on data from P, or its asymmetry.
- **gemma3:4b** · cross-entropy · `minimum-at-p-equals-q` → missing: The summary never mentions that cross-entropy is minimized when Q equals P, that the minimum is the entropy of P, or KL divergence.
- **gemma3:4b** · cross-entropy · `llm-pretraining-loss` → missing: It only says cross-entropy is used 'to train models' and does not state the average negative log probability of the true next token or that the log is forced.
- **gemma3:4b** · extreme-places · `five-places-covered` → missing: The summary covers only Greenland, Switzerland, Neom and Finland, omits Cyprus, and never presents the video as a compilation of five documentaries.
- **gemma3:4b** · future-of-capitalism · `decoupling-would-come-too-late` → missing: The summary does not mention decoupling or the 100-year versus 10-20 year timing.
- **gemma3:4b** · quantum-ai · `not-the-next-revolution` → missing: The summary only says quantum AI is in an early phase with 'potencial prometedor pero aún lejano', and never says it is not the next revolution replacing classical AI or that it is neither pure hype.
- **gemma3:4b** · stonewall-e4-knight · `dont-take-on-d5` → contradicted: It correctly says to avoid taking on d5 but also says the video "advocates for developing the bishop to g5", whereas the presenter says White should not play Bg5.
- **gemma3:4b** · stonewall-e4-knight · `f3-is-the-answer` → missing: The summary only mentions "preparing to trade pieces" and never mentions f3 or kicking the knight.

## Faithfulness: claims the video doesn't support

| model | major | minor | faithful summaries |
|---|---|---|---|
| qwen3:4b | 4 | 14 | 3/10 |
| gemma3:4b | 10 | 12 | 2/10 |

- **gemma3:4b** · accelerated-dragon · major misleading: "In this game against an opponent playing the Alapin Sicilian" → The video contains three separate games (starting 0:00, 17:12, 36:38) and only the third is against the Alapin (36:57), so presenting it as one game is wrong.
- **gemma3:4b** · accelerated-dragon · minor misleading: "Black initially developed the knight to C6, aiming for a standard queen takes D4 line" → The queen-takes-d4 transposition idea is from the first game (0:07-0:22), not the Alapin game, so details from different games are merged.
- **gemma3:4b** · accelerated-dragon · major unsupported: "faced some tactical challenges and a slightly passive position" → In the Alapin game the streamer says he was 'dominating this game' (57:03) and that he missed a win (42:57-43:17), never a passive position.
- **gemma3:4b** · accelerated-dragon · major misleading: "Black secured a draw through a series of exchanges and positional maneuvering, demonstrating a cautious approach" → The draw came after a missed win in a game the streamer dominated (57:00-57:15), and the other two games were won (13:51, 35:44), so the cautious-draw picture is wrong.
- **gemma3:4b** · accelerated-dragon · major wrong: "Black managed to hold onto an equal endgame" → The streamer was dominating the Alapin game (57:03) and aimed for a risk-free endgame (41:35), not just holding equality.
- **gemma3:4b** · accelerated-dragon · minor unsupported: "Despite a few moments of pressure" → The transcript shows Black applying the pressure on White, with no stretch of Black being under pressure.
- **qwen3:4b** · accelerated-dragon · major wrong: "ended in draws" → Only the third game is a draw (57:00 'Only made a draw'); the first is an 'easy win' (13:29) and in the second his pawns promoted (34:44).
- **qwen3:4b** · accelerated-dragon · major wrong: "resulting in a drawn position after careful maneuvering" → In the first game he wins the pawns and says 'this is an easy win' (13:29), finishing with 'Kaching' (13:51) and 'Keep the pawn' (16:14), not a draw.
- **qwen3:4b** · accelerated-dragon · minor wrong: "against an opponent who went for the Sicilian defense" → The streamer is Black and plays the Sicilian; the opponent (White) played d4 and an open-Sicilian-or-not line (0:03-0:15).
- **qwen3:4b** · accelerated-dragon · major wrong: "The second game involved a mistake by the creator that led to a draw despite initial advantages." → After the mistake the endgame was equal, but the opponent made errors and 'my pawns just promoted' (35:44), so the streamer won.
- **qwen3:4b** · accelerated-dragon · minor unsupported: "The creator concluded he learned from the experience but still found the opening challenging to play effectively." → No such conclusion is stated; he says he will keep going for this structure (45:33-45:55) and was 'dominating' the game (57:03).
- **gemma3:4b** · argentine-anarchism · major wrong: "Durante el periodo conservador argentino entre 1880 y 1916, el anarquismo experimentó su propia “Edad de Oro”" → The video presents the 1880-1916 golden age as an oligarchic narrative and places anarchism's golden age in the first years of the 20th century (0:00-0:40, 30:31-30:36).
- **gemma3:4b** · argentine-anarchism · minor misleading: "impulsado por las tensiones sociales y la represión gubernamental liderada por Ramón Falcón" → The video attributes the rise to social conditions, militancy and culture, while repression weakened the movement (21:39-21:56, 24:00-24:11), and Falcon is only the police chief.
- **gemma3:4b** · argentine-anarchism · minor unsupported: "La FORA, con iniciativas como *La Protesta* y *Ideas y Figuras*" → The video describes La Protesta and Ideas y Figuras as anarchist press and reviews, not as FORA initiatives (28:00, 29:01).
- **gemma3:4b** · argentine-anarchism · major unsupported: "la FORA fortaleció su influencia en el movimiento obrero, sentando las bases para futuras luchas" → The video says competition from other currents and repression reduced FORA's influence and displaced the anarchists from leadership (24:11-24:17, 31:05-31:13); 'bases for future struggles' is never said.
- **gemma3:4b** · argentine-anarchism · minor misleading: "consolidó el anarquismo como una fuerza significativa en la sociedad argentina" → Presents a purely consolidating outcome, whereas the video ends noting repression and a mid-term loss of FORA's influence (30:52-31:13).
- **qwen3:4b** · argentine-anarchism · minor wrong: "en el primer tercio del siglo XX" → The video places the golden age in the first years/first decade of the 20th century ([0:35], [30:47]), not the first third of the century.
- **gemma3:4b** · browser-tabs · minor misleading: "además de integrar funcionalidades como la automatización de releases con semantic release" → Semantic release is how the repo publishes its own releases from conventional commits, not a feature of the tab utility (4:18-4:58).
- **qwen3:4b** · browser-tabs · minor wrong: "El autor afirma que el tiempo invertido en su desarrollo no aumenta el costo operativo diario" → He says the utility itself should not increase Codely's baseline operating cost/workload day to day (3:50-4:15), not that the development time doesn't.
- **qwen3:4b** · browser-tabs · minor unsupported: "ayuda a ahorrar horas semanales en la preparación de los temas" → He only says it will save him 'mucho tiempo cada semana' (7:39-7:45), never quantifying hours.
- **qwen3:4b** · browser-tabs · minor misleading: "integra actualizaciones semánticas sin crear PRs adicionales" → He says semantic-release publishes releases automatically from conventional commits without a PR for the version bump (4:25-4:58), not 'semantic updates'.
- **gemma3:4b** · build-claude-code · minor unsupported: "el creador describe cómo implementa esto utilizando un nuevo tipo de agente" → The video implements it via a spawn tool that creates a Git worktree and starts an agent in a goroutine with monitoring hooks (7:51-9:30), not a 'new type of agent'.
- **gemma3:4b** · build-claude-code · minor misleading: "ofrece recursos adicionales, incluyendo un taller gratuito y un repositorio con ejercicios para practicar el “harness engineering”" → The free workshop is a separate Commit Academy RAG workshop (13:00-13:20), not a harness-engineering resource; only the repo/site has the exercises (10:36-10:43).
- **gemma3:4b** · cross-entropy · minor wrong: "effectively quantifying the information content of the data" → The video says cross-entropy measures average bits when a code optimized for one distribution is used on another (7:43-8:03, 14:44), while information content/entropy is a separate quantity (4:44, 7:27).
- **gemma3:4b** · cross-entropy · minor unsupported: "highlighting the surprising link between intelligence and efficient data representation" → The video only mentions 'compression is intelligence' as a phrase it will assess in a later part (30:01-30:05) and does not make that link in this video.
- **qwen3:4b** · cross-entropy · minor misleading: "concludes that this approach trains language models to compress text effectively, which underpins the idea that "compression is intelligence"" → The video only sets this up at the end (29:51-30:05) and defers the explanation and the assessment of 'compression is intelligence' to the next part, so presenting it as the video's conclusion overstates it.
- **gemma3:4b** · extreme-places · major misleading: "explores a diverse range of societies and locations defined by their extraordinary levels of preparedness" → The video is a compilation of five field films (Greenland, Switzerland, Neom, Finland, Cyprus) announced as the creator's favorites at 0:18, and preparedness is the theme only of the Switzerland and Finland segments, not of Greenland, Neom or Cyprus.
- **gemma3:4b** · extreme-places · major unsupported: "Ultimately, the video argues that these disparate examples – from Greenland to Finland – demonstrate humanity's capacity to adapt, prepare, and maintain resilience in the face of perceived threats and challenging environments." → The video makes no overarching argument; it is a compilation of separate films introduced as a top-five list (0:18), and it also includes a Cyprus film (2:26:53) that the summary omits while implying Finland is the last.
- **gemma3:4b** · extreme-places · minor misleading: "Greenland's vibrant cultural preservation amidst modernization" → The Greenland segment stresses Greenlandic culture being threatened and eroded by modernization (e.g. 31:18-31:35 'where Greenlandic culture goes to wither'), with only some traditions persisting (32:55).
- **qwen3:4b** · extreme-places · minor misleading: "Switzerland's bunker network successfully deterred Nazi aggression during WWII" → The video says the bunkers were only part of the deterrent, alongside Swiss banking and trade with the Nazis (48:52-49:09).
- **qwen3:4b** · extreme-places · minor unsupported: "explains its eight-year ranking as the happiest country" → The video only says preparedness 'might actually explain' the happiness ranking and offers other theories (coffee, saunas, wealth, belonging) (1:36:55; 2:24:36-2:25:04).
- **qwen3:4b** · extreme-places · minor wrong: "ghost towns abandoned for 47 years" → The video says one resort town, Famagusta, has been abandoned for 47 years (2:33:21), not multiple ghost towns.
- **gemma3:4b** · quantum-ai · major wrong: "la dificultad de procesar datos cuánticos" → The video says the difficulty is loading classical data into quantum states (19:00-19:23) and that processing data that is already quantum is especially promising and advantageous (19:26-20:21).
- **gemma3:4b** · quantum-ai · minor misleading: "el vídeo concluye que la IA cuántica se encuentra en una fase temprana de desarrollo, con un potencial prometedor pero aún lejano de ser una realidad generalizada" → The video's conclusion is that it is at an intermediate point, not hype and not the revolution replacing classical AI, with potential in complementing classical AI on specific tasks (20:24-21:04), not a general early-stage-but-coming framing.
- **qwen3:4b** · quantum-ai · minor misleading: "hasta que no se desarrollen ordenadores cuánticos más robustos, la IA cuántica no reemplazará a la clásica" → Video says quantum AI is definitely not the next revolution replacing classical AI, at least in the medium term (20:34-20:41), and only that its reach is hard to judge until more powerful computers exist (21:22-21:28), so the summary implies replacement once hardware improves.
- **gemma3:4b** · stonewall-e4-knight · major wrong: "the video advocates for developing the bishop to g5 strategically" → The presenter criticizes Bg5 ("what on earth is our bishop doing on G5?" at 1:36, "I do [hate bishop G5]" at 9:02) and prefers Bf4 or delaying the bishop (9:20-9:25).
- **qwen3:4b** · stonewall-e4-knight · major wrong: "makes the e4 knight manageable without trading it off" → The video says to be ready to trade into the knight, 'that's a trade that you want' (16:21), and that one valid route is to trade into it then play f3 (27:06-27:33), so 'without trading it off' misrepresents the conclusion.
- **qwen3:4b** · stonewall-e4-knight · minor unsupported: "avoiding the e4 knight trap through precise maneuvering" → The video never describes a 'trap' for Karpov to avoid; he kicks the knight with f3 and takes the dark squares (13:01-14:11).
- **qwen3:4b** · stonewall-e4-knight · minor wrong: "subsequent tactical play" → The video calls Karpov's game 'a masterpiece in terms of strategy' (15:56-16:00) and says the knight kick was followed by strategic play (27:36-27:43), not tactical play.
- **qwen3:4b** · stonewall-e4-knight · minor misleading: "due to positional errors that weaken tension" → The video says taking on d5 releases tension, gives up the center and frees Black's bishop (4:36-4:41, 5:11-5:31), rather than 'weakening tension' through errors.

## Code checks failed

- gemma3:4b · browser-tabs: direct_start
- gemma3:4b · build-claude-code: direct_start
- gemma3:4b · cross-entropy: direct_start
- qwen3:4b · extreme-places: length_3_to_5_sentences
- gemma3:4b · future-of-capitalism: direct_start
- qwen3:4b · future-of-capitalism: direct_start
- gemma3:4b · quantum-ai: direct_start
- qwen3:4b · stonewall-e4-knight: direct_start

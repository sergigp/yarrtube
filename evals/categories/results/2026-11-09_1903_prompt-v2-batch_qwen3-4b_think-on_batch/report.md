# Categorization eval: 2026-11-09_1903_prompt-v2-batch_qwen3-4b_think-on_batch

model `qwen3:4b`, thinking on, prompt v2-batch, temperature 0.0, num_ctx 16384, max output 12288, one call per video (--batch)

522 of 540 calls done.
2 videos have no summary and weren't asked about: run `--summarize`.

| category | labeled | found | missed | wrong | precision | recall | errors | pass |
|---|---|---|---|---|---|---|---|---|
| Jobava London | 5 | 4 | 1 | 0 | 4/4 | 4/5 | 1 | no |
| Sicilian | 7 | 6 | 0 | 0 | 6/6 | 6/6 | 1 | no |
| Chess gameplays | 13 | 12 | 0 | 1 | 12/13 | 12/12 | 1 | no |
| Science News | 9 | 8 | 1 | 1 | 8/9 | 8/9 | 1 | no |
| AI news | 10 | 7 | 3 | 1 | 7/8 | 7/10 | 1 | no |
| Tech | 4 | 2 | 2 | 0 | 2/2 | 2/4 | 1 | no |
| Politics and Economy News | 4 | 3 | 1 | 2 | 3/5 | 3/4 | 1 | no |
| Food for thought | 9 | 7 | 2 | 2 | 7/9 | 7/9 | 1 | no |
| Coding tutorials | 5 | 4 | 1 | 1 | 4/5 | 4/5 | 1 | no |

## Wrong: said yes, labeled no

- **Chess gameplays** · A Clean Engine Report Can Hide a Lot of Learning (ChessCoach Andras, random): The video is a detailed game review that walks through a specific chess game, making it a game review that fits the category of chess gameplays.
- **Science News** · What if you set off a nuke in the Mariana Trench? (xkcd's What If?, random): The video explains the physics of a nuclear explosion in the Mariana Trench and its potential consequences, fitting as an explainer on physics.
- **AI news** · The Design Process Behind This $18K/Month App (Starter Story Build, random): The video shows Ryan and Sarah using AI tools like Codex to build an app, which is a real case of software engineers working with AI, making it a hands-on review of AI tools.
- **Politics and Economy News** · The AI bubble is popping (David Ondrej, random): The video discusses current economic events such as stock market crashes due to AI buildout competition and government debt, which are the main subject of the video
- **Politics and Economy News** · Contra el colapso: futuro más allá del fin (Alba Lafarga, random): The video analyzes current political and economic issues, including energy transition policies and climate action, as a response to collapse narratives.
- **Food for thought** · How I Survived Late-Stage Cancer (Kurzgesagt – In a Nutshell, random): The video reflects on the human condition through the speaker's experience with cancer, changing their outlook on life.
- **Food for thought** · The world's most extreme places, explained (Johnny Harris, existing): The video provides long explanations of how extreme places (like Greenland, Switzerland, Saudi Arabia, Finland, and Cyprus) relate to cultural identity, historical trauma, economic projects, and geopolitical divisions, reflecting on society and the human condition.
- **Coding tutorials** · De 20 minutos a 1 segundo: Controla tu navegador desde terminal (CodelyTV - Redescubre la programación, existing): The video teaches how to use a CLI tool for managing browser tabs, which is a developer tool.

## Missed: said no, labeled yes

- **Jobava London** · This rare opening is virtually unstoppable below 2000 elo (IM Alex Banzea, picked): The video does not cover the Jobava London opening (d4, Nc3, Bf4) and instead focuses on Slav defense and other chess openings.
- **Science News** · How I Survived Late-Stage Cancer (Kurzgesagt – In a Nutshell, random): The video is a personal story about cancer survival, not a scientific explainer or news report on space/science.
- **AI news** · The AI bubble is popping (David Ondrej, random): The video is an opinion on AI's effect on society (market collapse) and the category explicitly excludes such opinions
- **AI news** · /wayfinder: Nothing is too big to plan anymore (Matt Pocock, random): The video is about a project management tool for software teams, not AI tools.
- **AI news** · La Aparente Inutilidad de la IA Cuántica (QuantumFracture, existing): The video discusses the theoretical limitations of quantum computing and AI, not AI tools or how people use them, so it does not belong.
- **Tech** · The AI bubble is popping (David Ondrej, random): The video is about AI market collapse and not about consumer technology products or the tech industry in the sense of gadgets or apps (the category excludes AI models and tools)
- **Tech** · The Design Process Behind This $18K/Month App (Starter Story Build, random): The video focuses on AI tools (excluded from Tech) rather than consumer technology products as the main subject, and it is not a product review.
- **Politics and Economy News** · The world's most extreme places, explained (Johnny Harris, existing): The video explains cultural and historical contexts of extreme places, not reporting on current political or economic news.
- **Food for thought** · Contra el colapso: futuro más allá del fin (Alba Lafarga, random): The video addresses current events (collapse narratives and energy policy) rather than timeless ideas about society and the human condition.
- **Food for thought** · If you put a toaster in a freezer, which wins? (xkcd's What If?, random): The video is an explainer on physics, not on society, the human condition, or philosophy.
- **Coding tutorials** · Compilation: 6 best FAANG system design mock interviews (ex-Google, Apple, Amazon, Meta engineers) (IGotAnOffer: Engineering, random): The video presents mock interviews for system design, not a step-by-step tutorial on programming or software engineering.

## Errors: no usable answer

- **Jobava London** · 1 in a Billion Checkmate? | ChessMood Speedrun Ep 13 (Eric Rosen, random): hit the 12288-token output limit
- **Sicilian** · 1 in a Billion Checkmate? | ChessMood Speedrun Ep 13 (Eric Rosen, random): hit the 12288-token output limit
- **Chess gameplays** · 1 in a Billion Checkmate? | ChessMood Speedrun Ep 13 (Eric Rosen, random): hit the 12288-token output limit
- **Science News** · 1 in a Billion Checkmate? | ChessMood Speedrun Ep 13 (Eric Rosen, random): hit the 12288-token output limit
- **AI news** · 1 in a Billion Checkmate? | ChessMood Speedrun Ep 13 (Eric Rosen, random): hit the 12288-token output limit
- **Tech** · 1 in a Billion Checkmate? | ChessMood Speedrun Ep 13 (Eric Rosen, random): hit the 12288-token output limit
- **Politics and Economy News** · 1 in a Billion Checkmate? | ChessMood Speedrun Ep 13 (Eric Rosen, random): hit the 12288-token output limit
- **Food for thought** · 1 in a Billion Checkmate? | ChessMood Speedrun Ep 13 (Eric Rosen, random): hit the 12288-token output limit
- **Coding tutorials** · 1 in a Billion Checkmate? | ChessMood Speedrun Ep 13 (Eric Rosen, random): hit the 12288-token output limit

## Speed

On the 58 videos asked about every category: 57.0s and 3580 output tokens per video on average (62.9 tokens/s here, input included), 58 calls in all.
At 11 tokens/s (the MacBook Air with thinking) that is about 5.4 min per video.

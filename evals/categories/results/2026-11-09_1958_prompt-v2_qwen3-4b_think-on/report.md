# Categorization eval: 2026-11-09_1958_prompt-v2_qwen3-4b_think-on

model `qwen3:4b`, thinking on, prompt v2, temperature 0.0, num_ctx 4096, max output 3072, one call per video + category

522 of 540 calls done.
2 videos have no summary and weren't asked about: run `--summarize`.

| category | labeled | found | missed | wrong | precision | recall | errors | pass |
|---|---|---|---|---|---|---|---|---|
| Jobava London | 5 | 4 | 1 | 0 | 4/4 | 4/5 | 2 | no |
| Sicilian | 7 | 6 | 0 | 0 | 6/6 | 6/6 | 6 | no |
| Chess gameplays | 13 | 13 | 0 | 1 | 13/14 | 13/13 | 0 | yes |
| Science News | 9 | 7 | 1 | 1 | 7/8 | 7/8 | 2 | no |
| AI news | 10 | 6 | 4 | 1 | 6/7 | 6/10 | 0 | yes |
| Tech | 4 | 2 | 1 | 1 | 2/3 | 2/3 | 2 | no |
| Politics and Economy News | 4 | 4 | 0 | 3 | 4/7 | 4/4 | 1 | no |
| Food for thought | 9 | 8 | 1 | 4 | 8/12 | 8/9 | 1 | no |
| Coding tutorials | 5 | 5 | 0 | 1 | 5/6 | 5/5 | 1 | no |

## Wrong: said yes, labeled no

- **Chess gameplays** · A Clean Engine Report Can Hide a Lot of Learning (ChessCoach Andras, random): The video provides a detailed game review and recap of a specific chess game, going through the moves and analyzing the mistakes, making the game the main content as required by the category.
- **Science News** · What if you set off a nuke in the Mariana Trench? (xkcd's What If?, random): The video is an explainer on physics regarding the effects of a nuclear explosion in the Mariana Trench, which fits the category's definition of 'explainers on physics'.
- **AI news** · The Design Process Behind This $18K/Month App (Starter Story Build, random): The video details the hands-on process of software engineers using AI tools like Procreate Dreams and Codex to build a $18,000/month app, making the AI work the central focus as required by the category's description.
- **Tech** · Compilation: 6 best FAANG system design mock interviews (ex-Google, Apple, Amazon, Meta engineers) (IGotAnOffer: Engineering, random): The video covers mock interviews where candidates design consumer products (Spotify, TikTok, messaging apps, Google Ads, Slack, and phone billing systems) for major tech companies, which falls under consumer technology and the tech industry as defined by the category description.
- **Politics and Economy News** · The AI bubble is popping (David Ondrej, random): The video reports on current economic news, specifically the collapse of the AI boom and its impact on stock markets and government debt, which are current economic events and policy decisions.
- **Politics and Economy News** · Contra el colapso: futuro más allá del fin (Alba Lafarga, random): The video analyzes current policy proposals for energy transition and their political implications to address climate change, which falls under economic news and policy decisions as the main subject.
- **Politics and Economy News** · SpaceX shows off the ship for the Moon | CDM News 🚀 (CDM Extra, random): The video reports on current geopolitical events including NASA's lunar mission plans, the competition for the first human lunar landing between Blue Origin and SpaceX, and a recent incident where a Russian satellite approached a Ukrainian satellite without authorization, generating space security concerns.
- **Food for thought** · What if you set off a nuke in the Mariana Trench? (xkcd's What If?, random): The video uses a hypothetical nuclear explosion scenario to illustrate the catastrophic societal consequences of small errors in scientific calculations, reflecting on the human condition and the importance of precision in critical fields, which aligns with the category's focus on lessons about society and the human condition.
- **Food for thought** · How I Survived Late-Stage Cancer (Kurzgesagt – In a Nutshell, random): The video reflects on the speaker's personal experience with late-stage cancer and its profound impact on their outlook on life, providing a philosophical lesson about the human condition that fits the category's focus on ideas about society and the human condition.
- **Food for thought** · The TERRIFYING REAL story of the man who fell from space | Vladimir Komarov (Control de Misión, random): The video covers the historical story of Vladimir Komarov's sacrifice during the Soyuz 1 mission in 1967 and reflects on the human condition through the legacy of space exploration, fitting the 'Food for thought' category as a historical lesson with philosophical implications.
- **Food for thought** · The world's most extreme places, explained (Johnny Harris, existing): The video provides long explanations of how extreme places (Greenland, Switzerland, Saudi Arabia, Finland, and Cyprus) have shaped societal structures and cultural identities through historical trauma and adaptation, reflecting on the human condition without focusing on current news events.
- **Coding tutorials** · De 20 minutos a 1 segundo: Controla tu navegador desde terminal (CodelyTV - Redescubre la programación, existing): The video teaches a CLI tool (browser tabs) for grouping and automating browser tab management in Vivaldi, which is a developer tool that fits under the category of coding tutorials and developer tools as described.

## Missed: said no, labeled yes

- **Jobava London** · This rare opening is virtually unstoppable below 2000 elo (IM Alex Banzea, picked): The video covers three chess games using openings such as Slav defense with H5, the 'Excalibur Knight' (knight B5), and handling Black's queen A5 threat, but does not discuss the Jobava London (White plays d4, Nc3, Bf4) as required by the category.
- **Science News** · How I Survived Late-Stage Cancer (Kurzgesagt – In a Nutshell, random): The video is a personal narrative about surviving testicular cancer and does not function as an explainer on medicine or a scientific news report as the category requires explainers on science topics like medicine, but this video is a first-person account rather than an educational explanation.
- **AI news** · The AI bubble is popping (David Ondrej, random): The video discusses the collapse of the AI boom and its societal and market implications, which is an opinion on AI's effect on society, explicitly excluded by the category description.
- **AI news** · /wayfinder: Nothing is too big to plan anymore (Matt Pocock, random): The video covers a non-AI project planning tool (Wayfinder) for software development without any mention of AI, so it does not fit the category's focus on AI tools and their usage.
- **AI news** · How to Test Your Code (Even When AI Writes It) (ByteByteGo, picked): The video covers software testing practices when AI writes code, but the main subject is testing (not the AI tools, workflows, or real cases of engineers working with AI as the primary focus), so it does not meet the category's requirement that the AI work is the main subject.
- **AI news** · La Aparente Inutilidad de la IA Cuántica (QuantumFracture, existing): The video discusses the practical limitations and theoretical potential of quantum AI, but it does not focus on real cases of software engineers working with AI tools, features of AI coding tools, or hands-on reviews as required by the category's description.
- **Tech** · The AI bubble is popping (David Ondrej, random): The video discusses the collapse of the AI boom with specific focus on AI models (Enthropic's Opus 5.5) and AI agents, which are explicitly excluded from the Tech category that covers consumer technology products and tech company news.
- **Food for thought** · If you put a toaster in a freezer, which wins? (xkcd's What If?, random): The video is a physics experiment on heat transfer between a toaster and freezer, which is a science topic and thus does not fit the category of 'Food for thought' that teaches or reflects on society and the human condition.

## Errors: no usable answer

- **Jobava London** · A Clean Engine Report Can Hide a Lot of Learning (ChessCoach Andras, random): hit the 3072-token output limit
- **Jobava London** · How to Deal With the e4 Knight in the Stonewall (ChessCoach Andras, existing): hit the 3072-token output limit
- **Sicilian** · I Coached This Team Before the European Club Cup (ChessCoach Andras, random): hit the 3072-token output limit
- **Sicilian** · Sir, Can I Have a Jobava London With Extra Tactics? (IM Alex Banzea, picked): hit the 3072-token output limit
- **Sicilian** · This Opening is a Glitch in the Chess Matrix (IM Alex Banzea, picked): hit the 3072-token output limit
- **Sicilian** · This rare opening is virtually unstoppable below 2000 elo (IM Alex Banzea, picked): hit the 3072-token output limit
- **Sicilian** · I Tried the Accelerated Dragon... Not What I Expected (IM Alex Banzea, existing): hit the 3072-token output limit
- **Sicilian** · How to Deal With the e4 Knight in the Stonewall (ChessCoach Andras, existing): hit the 3072-token output limit
- **Science News** · ALTMAN'S SINGULARITY is a LIE (Café Kyoto, random): hit the 3072-token output limit
- **Science News** · SpaceX shows off the ship for the Moon | CDM News 🚀 (CDM Extra, random): hit the 3072-token output limit
- **Tech** · The Design Process Behind This $18K/Month App (Starter Story Build, random): hit the 3072-token output limit
- **Tech** · If you put a toaster in a freezer, which wins? (xkcd's What If?, random): hit the 3072-token output limit
- **Politics and Economy News** · The Problem with Building One London (The B1M, random): hit the 3072-token output limit
- **Food for thought** · I tried Bryan Johnson's $2 MILLION anti-aging routine... on a $10 budget (Sara Klaus, random): hit the 3072-token output limit
- **Coding tutorials** · /wayfinder: Nothing is too big to plan anymore (Matt Pocock, random): hit the 3072-token output limit

## Speed

On the 58 videos asked about every category: 154.4s and 10706 output tokens per video on average (69.3 tokens/s here, input included), 522 calls in all.
At 11 tokens/s (the MacBook Air with thinking) that is about 16.2 min per video.

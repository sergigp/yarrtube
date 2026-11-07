# Categorization eval: 2026-11-06_1249_prompt-v2_qwen3-4b_think-on

model `qwen3:4b`, thinking on, prompt v2, temperature 0.0, num_ctx 4096

522 of 540 calls done.
2 videos have no summary and weren't asked about: run `--summarize`.

| category | labeled | found | missed | wrong | precision | recall | errors | pass |
|---|---|---|---|---|---|---|---|---|
| Jobava London | 5 | 2 | 0 | 0 | 2/2 | 2/2 | 7 | no |
| Sicilian | 6 | 5 | 0 | 1 | 5/6 | 5/5 | 8 | no |
| Chess gameplays | 13 | 11 | 0 | 3 | 11/14 | 11/11 | 2 | no |
| Science News | 9 | 0 | 7 | 0 | - | 0/7 | 2 | no |
| AI news | 10 | 3 | 2 | 1 | 3/4 | 3/5 | 5 | no |
| Tech | 4 | 2 | 1 | 0 | 2/2 | 2/3 | 4 | no |
| Politics and Economy News | 4 | 3 | 0 | 5 | 3/8 | 3/3 | 2 | no |
| Food for thought | 9 | 9 | 0 | 17 | 9/26 | 9/9 | 6 | no |
| Coding tutorials | 5 | 5 | 0 | 2 | 5/7 | 5/5 | 1 | no |

## Wrong: said yes, labeled no

- **Sicilian** · 1 in a Billion Checkmate? | ChessMood Speedrun Ep 13 (Eric Rosen, random): The video includes a game that is the Sicilian Grand Prix Attack, which is a Sicilian line explicitly listed in the category's belongs description.
- **Chess gameplays** · I Coached This Team Before the European Club Cup (ChessCoach Andras, random): The video is a detailed game review/recap of a specific chess game that the speaker guides the team through, including the move sequence and the outcome, which fits the category's description of 'game reviews or recaps that go through the moves'.
- **Chess gameplays** · A Clean Engine Report Can Hide a Lot of Learning (ChessCoach Andras, random): The video provides a detailed game walkthrough and analysis of a specific chess game to illustrate how engine reports can hide critical learning opportunities, which fits the category's requirement for game reviews or recaps that go through the moves.
- **Chess gameplays** · How to Deal With the e4 Knight in the Stonewall (ChessCoach Andras, existing): The video is a detailed recap of Karpov's game that goes through the moves to illustrate handling the e4 knight in the Stonewall opening, fitting the category as it provides a game review with move-by-move analysis.
- **AI news** · The Design Process Behind This $18K/Month App (Starter Story Build, random): The video details a real case of software engineers using AI tools like Codex to build an app, fitting the category's description of real cases of software engineers working with AI.
- **Politics and Economy News** · The AI bubble is popping (David Ondrej, random): The video discusses current economic events including the crashing stock markets and AI's impact on government debt and economic policy, which falls under economic news and policy decisions as defined by the category.
- **Politics and Economy News** · Contra el colapso: futuro más allá del fin (Alba Lafarga, random): The video analyzes current economic policy decisions, including the Greenpeace report's proposal for a 39% reduction in energy consumption by 2040 and the need for collective action in the energy transition to avoid 1.5°C of global warming, which falls under economic news and policy decisions.
- **Politics and Economy News** · SpaceX shows off the ship for the Moon | CDM News 🚀 (CDM Extra, random): The video covers current geopolitical events including NASA's lunar mission plans, the competition for the first human lunar landing, and a recent incident where a Russian satellite approached a Ukrainian satellite without authorization, which is tied to the ongoing war in Ukraine and falls under current wars and geopolitics as defined in the category.
- **Politics and Economy News** · The Problem with Building One London (The B1M, random): The video discusses the current economic and policy challenges of constructing One London, including strict aviation regulations that limit height to 309.6m to avoid airspace conflicts, which are direct policy decisions affecting the project's feasibility and timeline.
- **Politics and Economy News** · ¿Hay futuro en el capitalismo? (Alba Lafarga, existing): The video analyzes current economic policies (such as the Green New Deal and Fit for 55) and the capitalist system's sustainability in the context of climate agreements, which are current political and economic events covered by the category.
- **Food for thought** · What is OpenSpec and how does it work (CodelyTV - Redescubre la programación, random): The video explains the concept and process of OpenSpec, a technology framework for Spec Driven Development, which qualifies as an explainer on how technology works as per the category description.
- **Food for thought** · The Design Process Behind This $18K/Month App (Starter Story Build, random): The video teaches the design process and technology workflow for creating a human-like app using AI tools, which fits as an explainer on how technology works as per the category description.
- **Food for thought** · I tried Bryan Johnson's $2 MILLION anti-aging routine... on a $10 budget (Sara Klaus, random): The video reflects on the practicality and cost of anti-aging routines through a personal experiment, teaching viewers about the feasibility of high-end health regimens on a budget without being a news report or product review.
- **Food for thought** · What if you set off a nuke in the Mariana Trench? (xkcd's What If?, random): The video explains the physics of a nuclear explosion in the Mariana Trench and the catastrophic consequences of a calculation error, making it a scientific explainer that teaches ideas about physics and mathematics without being current events.
- **Food for thought** · How I Survived Late-Stage Cancer (Kurzgesagt – In a Nutshell, random): The video is a personal reflection on surviving cancer and its impact on life perspective, which fits the 'Food for thought' category as it reflects on philosophical ideas and life lessons.
- **Food for thought** · The TERRIFYING REAL story of the man who fell from space | Vladimir Komarov (Control de Misión, random): The video covers the historical account of Vladimir Komarov's sacrifice during the Soyuz 1 mission in 1967 and its significance in space exploration history, which fits the 'Food for thought' category as it belongs to the history domain as described.
- **Food for thought** · Why Does -1/12 Keep Appearing Everywhere In Physics? (PBS Space Time, random): The video provides a detailed explanation of how the mathematical result -1/12 appears in physics through regularization techniques in the Casimir effect and string theory, making it a long explainer on science and mathematics that fits the 'Food for thought' category.
- **Food for thought** · This DeepSeek v4.1 is powerful and uncensored, let's run it (David Ondrej, random): The video provides a detailed explanation of how to create uncensored AI models by modifying open-weight models like DeepSeek V4.1 Flash to reduce refusals, and introduces Cloud Room as a platform that boosts productivity, which is an explainer on technology work fitting the 'Food for thought' category as it teaches ideas without being current events, programming tutorials, product reviews, or chess.
- **Food for thought** · Professor Moriarty and the Physics of Holodecks - Sixty Symbols (Sixty Symbols, random): The video provides a detailed scientific explanation of the physics challenges in creating solid photonic characters for Star Trek's holodeck technology, focusing on quantum mechanics and the Pauli exclusion principle, which fits the category as it is an explainer on how science works.
- **Food for thought** · /wayfinder: Nothing is too big to plan anymore (Matt Pocock, random): The video explains a planning tool for large-scale software projects that maps decisions across sessions to handle ambiguity, fitting the category as an explainer of technology work without being a programming tutorial or product review.
- **Food for thought** · Grok Bot is more useful than I thought (Brian Casel, random): The video explains how Brian Castle designs and implements a system using AI bots for autonomous business operations, reflecting on the principles of technology and system design that are portable across AI platforms, fitting the category as an explainer on technology.
- **Food for thought** · Un Telescopio Bajo el Hielo: El Nobel 2026 EXPLICADO (QuantumFracture, picked): The video provides a detailed explanation of the IceCube neutrino telescope project and its scientific significance, functioning as an explainer on how a technology (neutrino detection) works in physics, which aligns with the 'Food for thought' category's definition of long explanations on science.
- **Food for thought** · System design walkthrough: How to design a rate limiter, according to an ex-Apple EM (IGotAnOffer: Engineering, picked): The video provides a detailed explanation of designing a distributed rate limiter system, which fits as an explainer on how technology works as described in the category's 'explainers on how science, mathematics or technology work' inclusion.
- **Food for thought** · How to Test Your Code (Even When AI Writes It) (ByteByteGo, picked): The video covers the concept of software testing layers and the implications of AI in code development, which is an explainer on how technology works and thus fits the 'Food for thought' category.
- **Food for thought** · Creé mi propio Claude Code (para aprender) (BettaTech, existing): The video explains how to build AI agents and harness interfaces in Go as a conceptual explainer for demystifying AI engineering, fitting the category's definition of explainers on how technology works.
- **Food for thought** · But what is cross-entropy? | Compression is Intelligence Part 2 (3Blue1Brown, existing): The video provides a detailed explanation of cross-entropy as a bridge between information theory and machine learning, demonstrating how compression techniques underpin language model training and the concept of 'compression is intelligence', fitting the category as an explainer on science and technology.
- **Food for thought** · La Aparente Inutilidad de la IA Cuántica (QuantumFracture, existing): The video provides a detailed explanation of quantum AI's current theoretical applications and limitations, reflecting on its role as a specialized tool in scientific niches rather than practical uses, which fits the 'Food for thought' category as an explainer on how science and technology work.
- **Coding tutorials** · This DeepSeek v4.1 is powerful and uncensored, let's run it (David Ondrej, random): The video provides a tutorial on modifying AI models to reduce refusals and introduces Cloud Room, a developer tool for running AI models, which fits the category as it teaches software engineering tasks and developer tools.
- **Coding tutorials** · De 20 minutos a 1 segundo: Controla tu navegador desde terminal (CodelyTV - Redescubre la programación, existing): The video teaches how to use a CLI tool (browser tabs) for organizing browser tabs in Vivaldi, which is a developer tool that automates workflow tasks for developers, fitting the category description of developer tools.

## Missed: said no, labeled yes

- **Science News** · How I Survived Late-Stage Cancer (Kurzgesagt – In a Nutshell, random): The video is a personal account of surviving testicular cancer treatment, which is a medical story and not a report on recent events in space or science as defined by the category's examples (astronomy, SpaceX, NASA missions, rocket launches, new discoveries, research results, Nobel Prizes) or recent scientific events.
- **Science News** · If you put a toaster in a freezer, which wins? (xkcd's What If?, random): The video is a physics thought experiment explaining why a toaster would work in a freezer without reporting recent events in space or science, fitting the category's exclusion of explainers with no recent event.
- **Science News** · The TERRIFYING REAL story of the man who fell from space | Vladimir Komarov (Control de Misión, random): The video covers the historical death of cosmonaut Vladimir Komarov during the 1967 Soyuz 1 mission, which is a past event and falls under history, not recent space events as required by the Science News category.
- **Science News** · Why Does -1/12 Keep Appearing Everywhere In Physics? (PBS Space Time, random): The video explains the mathematical result -1/12 and its physical implications in established physics contexts without reporting on recent events, missions, discoveries, or awards, making it an explainer with no recent event as defined by the category's exclusion.
- **Science News** · Professor Moriarty and the Physics of Holodecks - Sixty Symbols (Sixty Symbols, random): The video is an explainer on the physics challenges of creating solid photonic characters in Star Trek's holodeck technology without reporting any recent event, thus falling under the category's exclusion of 'explainers on how things work with no recent event'.
- **Science News** · But what is cross-entropy? | Compression is Intelligence Part 2 (3Blue1Brown, existing): The video is an explainer on cross-entropy and its application in language model training without reporting on recent events in space or science, fitting the category's exclusion of 'explainers on how things work with no recent event'.
- **Science News** · La Aparente Inutilidad de la IA Cuántica (QuantumFracture, existing): The video explains the current theoretical state of quantum AI and quantum computing without reporting recent scientific events, thus falling under the category's exclusion of explainers with no recent event.
- **AI news** · The AI bubble is popping (David Ondrej, random): The video discusses the collapse of the AI boom and societal implications, which is an opinion on AI's effect on society (explicitly excluded by the category description), and does not cover the specific topics that belong (new models in a comparative context, features of AI coding tools, AI workflows, hands-on reviews, or real cases of software engineers).
- **AI news** · /wayfinder: Nothing is too big to plan anymore (Matt Pocock, random): The video covers a project management tool called Wayfinder for planning large-scale work without AI, not AI tools or their usage by software engineers.
- **Tech** · The AI bubble is popping (David Ondrej, random): The video discusses the collapse of the AI boom with specific focus on AI models (Enthropic's Opus 5.5) and AI agents, which are explicitly excluded from the Tech category that covers consumer technology products and tech company news.

## Errors: no usable answer

- **Jobava London** · I Coached This Team Before the European Club Cup (ChessCoach Andras, random): unusable answer: 
- **Jobava London** · A Clean Engine Report Can Hide a Lot of Learning (ChessCoach Andras, random): unusable answer: 
- **Jobava London** · This rare opening is virtually unstoppable below 2000 elo (IM Alex Banzea, picked): unusable answer: {"reason": "The video covers three chess games using openings such as Slav defense with H5, the 'Excalibur Knight' (knight B5), and handling Black's queen A5 threat, but does not discuss the Jobava London (White plays d4, Nc3, Bf
- **Jobava London** · Crushing Through the Kingside | Jobava London | GM Naroditsky’s Theory Speed Run (Daniel Naroditsky, picked): unusable answer: 
- **Jobava London** · Rare & Venomous TRAP! | ChessMood Speedrun Ep 11 (Eric Rosen, picked): unusable answer: 
- **Jobava London** · I Tried the Accelerated Dragon... Not What I Expected (IM Alex Banzea, existing): unusable answer: 
- **Jobava London** · How to Deal With the e4 Knight in the Stonewall (ChessCoach Andras, existing): unusable answer: 
- **Sicilian** · I Coached This Team Before the European Club Cup (ChessCoach Andras, random): unusable answer: 
- **Sicilian** · Chess Tactics: Spot the Trigger, Not Just the Move (Dr. Can's Chess Clinic, random): unusable answer: {"reason": "The video covers general chess tactics and does not focus on the Sicilian Defense (1.e
- **Sicilian** · A Clean Engine Report Can Hide a Lot of Learning (ChessCoach Andras, random): unusable answer: 
- **Sicilian** · Sir, Can I Have a Jobava London With Extra Tactics? (IM Alex Banzea, picked): unusable answer: 
- **Sicilian** · This Opening is a Glitch in the Chess Matrix (IM Alex Banzea, picked): unusable answer: 
- **Sicilian** · This rare opening is virtually unstoppable below 2000 elo (IM Alex Banzea, picked): unusable answer: 
- **Sicilian** · I Tried the Accelerated Dragon... Not What I Expected (IM Alex Banzea, existing): unusable answer: 
- **Sicilian** · How to Deal With the e4 Knight in the Stonewall (ChessCoach Andras, existing): unusable answer: 
- **Chess gameplays** · Crushing Through the Kingside | Jobava London | GM Naroditsky’s Theory Speed Run (Daniel Naroditsky, picked): unusable answer: 
- **Chess gameplays** · Finding Powerful Squares!! | Rossolimo, Sicilian Defense | Top Theory Speedrun | GM Naroditsky (Daniel Naroditsky, picked): unusable answer: 
- **Science News** · SpaceX shows off the ship for the Moon | CDM News 🚀 (CDM Extra, random): unusable answer: 
- **Science News** · Un Telescopio Bajo el Hielo: El Nobel 2026 EXPLICADO (QuantumFracture, picked): unusable answer: 
- **AI news** · kimi k3.1 changes everything... (David Ondrej, random): unusable answer: 
- **AI news** · This DeepSeek v4.1 is powerful and uncensored, let's run it (David Ondrej, random): unusable answer: 
- **AI news** · Grok Bot is more useful than I thought (Brian Casel, random): unusable answer: 
- **AI news** · Creé mi propio Claude Code (para aprender) (BettaTech, existing): unusable answer: 
- **AI news** · La Aparente Inutilidad de la IA Cuántica (QuantumFracture, existing): unusable answer: 
- **Tech** · The Design Process Behind This $18K/Month App (Starter Story Build, random): unusable answer: 
- **Tech** · Compilation: 6 best FAANG system design mock interviews (ex-Google, Apple, Amazon, Meta engineers) (IGotAnOffer: Engineering, random): unusable answer: 
- **Tech** · If you put a toaster in a freezer, which wins? (xkcd's What If?, random): unusable answer: 
- **Tech** · SpaceX shows off the ship for the Moon | CDM News 🚀 (CDM Extra, random): unusable answer: 
- **Politics and Economy News** · ALTMAN'S SINGULARITY is a LIE (Café Kyoto, random): unusable answer: 
- **Politics and Economy News** · The world's most extreme places, explained (Johnny Harris, existing): unusable answer: 
- **Food for thought** · The AI bubble is popping (David Ondrej, random): unusable answer: 
- **Food for thought** · Compilation: 6 best FAANG system design mock interviews (ex-Google, Apple, Amazon, Meta engineers) (IGotAnOffer: Engineering, random): unusable answer: 
- **Food for thought** · I tested 4 AI agents on 9 real tasks. Here’s who won. (How I AI, random): unusable answer: 
- **Food for thought** · The Problem with Building One London (The B1M, random): unusable answer: 
- **Food for thought** · US Election Results Are In (Felu Live, picked): unusable answer: 
- **Food for thought** · The world's most extreme places, explained (Johnny Harris, existing): unusable answer: 
- **Coding tutorials** · /wayfinder: Nothing is too big to plan anymore (Matt Pocock, random): unusable answer: 

## Speed

Average 99.9s per call, slowest 690.4s. The full 540 calls take about 15.0 hours.

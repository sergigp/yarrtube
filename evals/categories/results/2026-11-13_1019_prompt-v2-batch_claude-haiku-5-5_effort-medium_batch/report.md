# Categorization eval: 2026-11-13_1019_prompt-v2-batch_claude-haiku-5-5_effort-medium_batch

model `claude-haiku-5-5` on Anthropic, effort medium, adaptive thinking, prompt v2-batch, max output scaled to the categories asked, one call per video (--batch), each category left without an answer asked alone

522 of 540 calls done.
2 videos have no summary and weren't asked about: run `--summarize`.

| category | labeled | found | missed | wrong | precision | recall | errors | pass |
|---|---|---|---|---|---|---|---|---|
| Jobava London | 5 | 4 | 1 | 0 | 4/4 | 4/5 | 0 | yes |
| Sicilian | 7 | 7 | 0 | 0 | 7/7 | 7/7 | 0 | yes |
| Chess gameplays | 13 | 13 | 0 | 1 | 13/14 | 13/13 | 0 | yes |
| Science News | 9 | 9 | 0 | 1 | 9/10 | 9/9 | 0 | yes |
| AI news | 10 | 7 | 3 | 2 | 7/9 | 7/10 | 0 | no |
| Tech | 4 | 2 | 2 | 0 | 2/2 | 2/4 | 0 | yes |
| Politics and Economy News | 4 | 4 | 0 | 0 | 4/4 | 4/4 | 0 | yes |
| Food for thought | 9 | 8 | 1 | 2 | 8/10 | 8/9 | 0 | no |
| Coding tutorials | 5 | 5 | 0 | 3 | 5/8 | 5/5 | 0 | no |

## Wrong: said yes, labeled no

- **Chess gameplays** · A Clean Engine Report Can Hide a Lot of Learning (ChessCoach Andras, random): The video reviews a specific game move by move in its walkthrough and mistake-analysis segments, which is a game review going through the moves.
- **Science News** · What if you set off a nuke in the Mariana Trench? (xkcd's What If?, random): The video explains what would happen physically if a nuclear bomb exploded in the Mariana Trench, covering the physics of steam cavities and waves, so it is a science explainer.
- **AI news** · What is OpenSpec and how does it work (CodelyTV - Redescubre la programación, random): OpenSpec is a development framework that is compatible with AI tools and is evaluated for usefulness with them, so AI tooling is a real part of the subject, though the main focus is the spec-driven development process itself.
- **AI news** · The Design Process Behind This $18K/Month App (Starter Story Build, random): A real segment covers building the app with Codex, including a master AI prompt and tips for designing and building with AI, so AI workflows are a substantial part of the video.
- **Food for thought** · How I Survived Late-Stage Cancer (Kurzgesagt – In a Nutshell, random): The video is a reflective personal account about surviving cancer and how it changed the speaker's outlook on life, which touches on the human condition, though its main focus is medical and personal experience rather than society, history, or philosophy.
- **Food for thought** · The world's most extreme places, explained (Johnny Harris, existing): Large parts explain society, culture and history, such as Greenlanders preserving cultural identity, Switzerland's wartime bunkers and Cyprus's division, as long explanations rather than news.
- **Coding tutorials** · The Design Process Behind This $18K/Month App (Starter Story Build, random): A segment walks through building the app with Codex and a prompt, which counts as a walkthrough of building software.
- **Coding tutorials** · /wayfinder: Nothing is too big to plan anymore (Matt Pocock, random): The video explains how to use Wayfinder, which creates specs and tickets in an issue tracker for planning software projects, and it works as a software development workflow walkthrough.
- **Coding tutorials** · De 20 minutos a 1 segundo: Controla tu navegador desde terminal (CodelyTV - Redescubre la programación, existing): The video presents a open-source CLI tool built by Codely for terminal-based browser tab management, including how it groups tabs and integrates semantic updates, which fits developer tools and software building.

## Missed: said no, labeled yes

- **Jobava London** · This rare opening is virtually unstoppable below 2000 elo (IM Alex Banzea, picked): The summary mentions a Slav defense, Black's queen on a5 and a knight on b5, but never mentions d4, Nc3 or Bf4, so the details don't show the Jobava London.
- **AI news** · /wayfinder: Nothing is too big to plan anymore (Matt Pocock, random): Wayfinder creates tickets in an issue tracker to plan work, and while the video is about planning, it does not center on AI tools or how people use them, so it does not fit.
- **AI news** · How to Test Your Code (Even When AI Writes It) (ByteByteGo, picked): AI is only the context for a discussion of testing strategy; the video does not review AI tools or models, and its main subject is testing rather than AI.
- **AI news** · La Aparente Inutilidad de la IA Cuántica (QuantumFracture, existing): The video discusses the theoretical potential and limits of quantum AI rather than specific AI tools, models, or how people use them, which the category excludes as academic theory.
- **Tech** · The AI bubble is popping (David Ondrej, random): The video is about AI models and AI tools, which the Tech category excludes, and it does not cover consumer tech products or company news.
- **Tech** · The Design Process Behind This $18K/Month App (Starter Story Build, random): The video is a how-to guide on designing and building an app, not consumer tech news or a product review.
- **Food for thought** · If you put a toaster in a freezer, which wins? (xkcd's What If?, random): The video is a physics-based hypothetical about appliances, not reflection on society, history, or philosophy, and it fits the science explainer category instead.

## Speed

On the 58 videos asked about every category: 3.3s and 594 output tokens per video on average (181.7 tokens/s here, input included), 58 calls in all.

## Tokens

claude-haiku-5-5: 58 calls, 174,394 input + 34,432 output tokens, about $0.035

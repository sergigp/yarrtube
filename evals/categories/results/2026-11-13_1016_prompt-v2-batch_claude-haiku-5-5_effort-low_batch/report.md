# Categorization eval: 2026-11-13_1016_prompt-v2-batch_claude-haiku-5-5_effort-low_batch

model `claude-haiku-5-5` on Anthropic, effort low, adaptive thinking, prompt v2-batch, max output scaled to the categories asked, one call per video (--batch), each category left without an answer asked alone

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
| Food for thought | 9 | 7 | 2 | 1 | 7/8 | 7/9 | 0 | yes |
| Coding tutorials | 5 | 5 | 0 | 1 | 5/6 | 5/5 | 0 | yes |

## Wrong: said yes, labeled no

- **Chess gameplays** · A Clean Engine Report Can Hide a Lot of Learning (ChessCoach Andras, random): The video centers on a single game with a walkthrough of the moves (game walkthrough chapter and deep analysis of mistakes), so the games are a main part of the content.
- **Science News** · What if you set off a nuke in the Mariana Trench? (xkcd's What If?, random): The video explains the physics of an underwater nuclear blast and its hypothetical consequences, which is a science explainer.
- **AI news** · What is OpenSpec and how does it work (CodelyTV - Redescubre la programación, random): OpenSpec is a spec-driven development framework that is compatible with AI tools and is evaluated for usefulness with them, so AI tools are a real part of the subject, though the main focus is the development process.
- **AI news** · The Design Process Behind This $18K/Month App (Starter Story Build, random): The video is a walkthrough of building an app using AI tools like Codex and an AI master prompt, with AI workflow as a main subject.
- **Food for thought** · The world's most extreme places, explained (Johnny Harris, existing): The video explains the history and culture of Greenland, Switzerland, Finland and Cyprus, reflecting on society and the human condition in long explanations rather than on current events alone.
- **Coding tutorials** · De 20 minutos a 1 segundo: Controla tu navegador desde terminal (CodelyTV - Redescubre la programación, existing): The video shows how to build and use a developer CLI tool built by Codely, with open source code on GitHub and NPM, which fits developer tools and software building.

## Missed: said no, labeled yes

- **Jobava London** · This rare opening is virtually unstoppable below 2000 elo (IM Alex Banzea, picked): The video's title and summary focus on a rare opening, and the description says it is the Slav with H5 and a Excalibur Knight line, not the Jobava London (1.d4, Nc3, Bf4); no Jobava setup is indicated.
- **AI news** · /wayfinder: Nothing is too big to plan anymore (Matt Pocock, random): Wayfinder creates tickets for research, prototypes, and tasks in an issue tracker, but the video is about a planning method rather than AI tools or how people use them, so AI is not the main subject.
- **AI news** · How to Test Your Code (Even When AI Writes It) (ByteByteGo, picked): The video discusses how AI-written code affects testing, which touches on AI in software development, but its main subject is testing methodology rather than AI tools, models or news about them.
- **AI news** · La Aparente Inutilidad de la IA Cuántica (QuantumFracture, existing): The video discusses quantum AI as a technology, its potential advantages and current limits, but it is not about AI tools, new models, or how people use them.
- **Tech** · The AI bubble is popping (David Ondrej, random): The video is about AI models and AI tools, which the Tech category explicitly excludes, and about market dynamics rather than consumer tech.
- **Tech** · The Design Process Behind This $18K/Month App (Starter Story Build, random): The video covers designing and building an iOS app for the App Store, but its focus is a how-to process rather than consumer tech news or reviews, and AI tools are excluded from Tech.
- **Food for thought** · If you put a toaster in a freezer, which wins? (xkcd's What If?, random): The video is a physical science hypothetical, not a lesson on society, history, or philosophy.
- **Food for thought** · You Will Only Like Half of This Video (Kurzgesagt – In a Nutshell, random): The video reflects on human creativity and critical thinking in the age of AI, which touches on the human condition, but it is mainly a channel message and promotion rather than a lesson or essay on society, history or philosophy.

## Speed

On the 58 videos asked about every category: 2.8s and 482 output tokens per video on average (170.8 tokens/s here, input included), 58 calls in all.

## Tokens

claude-haiku-5-5: 58 calls, 174,394 input + 27,985 output tokens, about $0.031

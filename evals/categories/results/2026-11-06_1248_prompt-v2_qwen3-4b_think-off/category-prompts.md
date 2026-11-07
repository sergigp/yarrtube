# Category prompts

The category prompts under evaluation. They are the owner's category data, not
shipped defaults; the system prompt is [`prompts/categorization/v2.md`](../../prompts/categorization/v2.md).

## User message draft

Built by code, one call per video + category. Where it lives is decided when
categorization is integrated.

```
Title: {title}
Channel: {channel}
Duration: {minutes} minutes
Chapters:
{chapter_list}
Summary: {summary}

Category: {name}
Description:
{category_prompt}

Answer in JSON: "reason", one sentence on what the video covers and how it fits, then "belongs", true or false.
```

Response schema, sent as Ollama's `format`:

```json
{
  "type": "object",
  "properties": {
    "reason": {"type": "string"},
    "belongs": {"type": "boolean"}
  },
  "required": ["reason", "belongs"]
}
```

## Jobava London

```
The video is about the Jobava London, also called the Rapport-Jobava London: White plays d4, Nc3 and Bf4. One game in a stream is enough.
Belongs: games by either side, streams, speedruns, game reviews, traps, plans and repertoire lessons.
Does not belong: the regular London System (Nf3, e3, c3), the Veresov (Nc3 with Bg5), or videos that only mention the opening.
```

## Sicilian

```
The video is about the Sicilian Defense (1.e4 c5). One game in a stream is enough.
Belongs: games by either side, streams, game reviews, traps and lessons in any Sicilian line, including Najdorf, Dragon, Accelerated Dragon, Sveshnikov, Taimanov, Kan, Scheveningen, Alapin, Smith-Morra, Grand Prix, Rossolimo, Open and Closed Sicilian.
Does not belong: other openings, or videos that only mention the Sicilian.
```

## Chess gameplays

```
The video shows people playing chess: the games are the main content.
Belongs: streams, speedruns, online games, tournament games, and game reviews or recaps that go through the moves.
Does not belong: chess news without games, opening lessons or trap videos without full games, or content about chess players' lives.
```

## Science News

```
The video reports on recent events in space and science.
Belongs: astronomy news, SpaceX and NASA missions, rocket launches, new discoveries, research results, and the Nobel Prizes and other science awards.
Does not belong: explainers on how things work with no recent event, software, AI products, gadgets, politics or history.
```

## AI news

```
The video is about AI tools and how people use them.
Belongs: new models and how they compare, features of AI coding tools such as Claude Code, Codex and Pi, AI workflows, hands-on reviews, and real cases of software engineers working with AI.
Does not belong: programming without AI, the academic theory behind AI, or opinions on AI's effect on society.
```

## Tech

```
The video is about consumer technology and the tech industry, not about code.
Belongs: Mac and Apple news, phones, gadgets, apps, operating system updates, product reviews and tech company news.
Does not belong: programming, tech tutorials or how-to guides, explainers on how technology works, or AI models and AI tools.
```

## Politics and Economy News

```
The video covers current political or economic events.
Belongs: elections, governments, geopolitics, current wars, economic news and policy decisions.
Does not belong: history or theory that isn't tied to current events, or celebrity and crime news.
```

## Food for thought

```
The video teaches or reflects on ideas, not on current events.
Belongs: history, sociology, economics, religion, philosophy, culture, and explainers on how science, mathematics or technology work, as lessons, essays or long explanations.
Does not belong: news, programming tutorials, product reviews or chess.
```

## Coding tutorials

```
The video teaches programming or software engineering.
Belongs: coding tutorials, programming languages, frameworks, software architecture, developer tools such as the terminal, Git or editors, and walkthroughs of building software.
Does not belong: tech news, product reviews, or news and reviews of AI tools.
```

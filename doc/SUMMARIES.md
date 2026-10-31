# Summaries: first benchmark (2026-10-30)

Raw output of the first summary benchmark, kept as reference for prompt and
model comparisons. Setup, prompt and timings are in [AI.md](AI.md)
("Findings: first summary benchmark"). Thinking qwen3 output has its
`<think>` reasoning stripped. Only the errors marked below were verified
against the transcripts; unmarked summaries were not fact-checked.

## ¿Hay futuro en el capitalismo-

`S2026E020101` · es · 16 min

### gemma3:4b

41.2 s · 150 output tokens

Este video analiza la situación actual de las políticas climáticas, destacando su fragilidad debido al resurgimiento de negacionismos y la desinformación. Se examina cómo el ascenso de gobiernos de derecha y el nacionalismo han dificultado la cooperación global, alejándose de la solidaridad planetaria necesaria para abordar el cambio climático. El video explora las ideas de futuro que han estado en juego, como el crecimiento verde y la innovación tecnológica, pero critica la excesiva confianza en soluciones tecnológicas sin cuestionar los patrones de consumo. Finalmente, se argumenta que el capitalismo, tal como está, no ofrece una solución sostenible y se proponen alternativas como el decrecimiento y una planificación democrática de la tecnología para garantizar un futuro viable.

### qwen3:4b (thinking)

130.6 s · 1923 output tokens

El video analiza cómo las políticas climáticas recientes, como el Acuerdo de París, intentan adaptar el capitalismo a la crisis climática sin cuestionar su modelo de crecimiento, destacando compromisos flexibles y mecanismos como los mercados de carbono. Critica la externalización de costes ambientales a países en desarrollo y la dependencia de tecnologías futuras para reducir emisiones, señalando que estos enfoques perpetúan desigualdades y no abordan las raíces del problema. Concluye que evitar escenarios irreversibles requiere abandonar el crecimiento ilimitado y adoptar modelos de decrecimiento y equidad, ya que el capitalismo actual no es compatible con los límites planetarios.

### qwen3:4b-instruct

46.6 s · 203 output tokens

El video analiza las políticas climáticas globales desde una perspectiva crítica, destacando que el capitalismo verde, aunque domina en acuerdos como el de París, no cuestiona sus fundamentos de crecimiento ilimitado y explotación de recursos. Se expone que estas propuestas dependen excesivamente de tecnologías futuras y externalizan los costos ambientales a países en desarrollo, perpetuando desigualdades. El autor argumenta que el crecimiento económico es incompatible con los límites planetarios, y propone alternativas como el decrecimiento, el ecomodernismo o el crecimiento tecnológico selectivo. Se concluye que el futuro del planeta no depende solo de tecnologías, sino de cambiar las ideas sobre la sociedad, la economía y la relación con la naturaleza. La transición climática es, en esencia, una disputa sobre qué tipo de sociedad queremos mantener.

## The GOLDEN AGE of ARGENTINE ANARCHISM

`S2026E063001` · es · 32 min

### gemma3:4b

59.3 s · 166 output tokens

> ⚠️ Calls 1880–1916 the golden age of anarchism; the video says that is the oligarchy's narrative.

Este video explora el período conocido como la “Edad de Oro” del anarquismo en Argentina, entre 1880 y 1916, un momento en el que la oligarquía local buscó justificar su poder. Durante este tiempo, el movimiento obrero anarquista experimentó un auge significativo, facilitado por las condiciones sociopolíticas impuestas por el orden conservador. La FORA, como resultado de un proceso de crecimiento de las organizaciones sindicales, se convirtió en un actor clave, difundiendo ideas libertarias y desafiando el sistema capitalista y estatal.  El video analiza cómo la influencia del anarquismo se extendió a través de la cultura y la organización gremial, y cómo el Estado respondió con medidas represivas para contener este movimiento.

### qwen3:4b (thinking)

242.6 s · 2560 output tokens

Argentina vivió su época de oro del movimiento anarquista en los primeros años del siglo XX, contrarrestando la narrativa oligárquica que destacó el periodo conservador de 1880 a 1916. Durante esta etapa, la FORA creció de 27 a casi 100 sindicatos en seis años, logrando avances laborales como reducciones de horas y mejoras salariales, mientras que la represión estatal incluyó la Ley de Residencia y la semana roja de 1909. Los anarquistas promovieron una amplia difusión cultural mediante periódicos, escuelas y actividades artísticas, pero el Estado y la rivalidad con otras corrientes obreras redujeron su influencia en la conducción del movimiento a mediano plazo. Este periodo marcó un auge ideológico y organizativo para el anarquismo argentino, a pesar de la intensa represión y el desplazamiento posterior en la lucha obrera.

### qwen3:4b-instruct

110.3 s · 293 output tokens

> ⚠️ "Más de 100 sindicatos en 1905": the transcript says almost 100.

La Argentina vivió durante el siglo XX una época de oro para el movimiento obrero anarquista, especialmente entre 1901 y 1916, cuando las condiciones sociales y económicas, junto con la exclusión sistemática de la élite, facilitaron el arraigo de sus ideas. La FORA creció rápidamente, con más de 100 sindicatos en 1905, y promovió mejoras laborales como la reducción de horas de trabajo y el fin de castigos físicos, aunque enfrentó fuertes represiones por parte del Estado. A través de medios culturales como periódicos, círculos de estudio, música y literatura, los anarquistas difundieron sus ideas y fortalecieron el espíritu crítico entre los trabajadores, especialmente en contextos de exclusión social y machismo tradicional. Este proceso de expansión y lucha, aunque fue interrumpido por represiones como la semana roja de 1909 y la ley de residencia, consolidó la FORA como un espacio de resistencia y pensamiento emancipador. La influencia del anarquismo se extendió más allá de los sindicatos, impactando en la educación, la cultura y la vida cotidiana de los trabajadores.

## But what is cross-entropy- - Compression is Intelligence Part 2

`S2026E071601` · en · 33 min

### gemma3:4b

59.1 s · 148 output tokens

This YouTube video explores the surprising connection between text compression and the training of language models. It details a 2002 paper that used gzip compression to automatically cluster languages and recover their relationships, demonstrating that the key metric was a distance based on compression efficiency. The core concept is cross-entropy, which measures the difference between a model’s predicted probability distribution and the true distribution of the data – effectively quantifying how much information is lost. This approach, originally conceived for compression, is now a fundamental loss function in training large language models, particularly through distillation, where a smaller model learns to mimic a larger one’s predictions. Ultimately, the video argues that compression principles offer a surprisingly insightful framework for understanding and improving language model training.

### qwen3:4b (thinking)

99.2 s · 664 output tokens

The video explains how cross-entropy naturally emerges from file compression principles, demonstrating its application in a 2002 paper that used gzip to cluster languages and reconstruct linguistic relationships without prior linguistic knowledge. It shows that cross-entropy measures how efficiently a compression scheme optimized for one context performs when encountering a different context, such as comparing text documents. In modern language models, this same concept forms the loss function that trains models to predict text by minimizing the difference between predicted and actual token probabilities. The core insight reveals that using cross-entropy loss effectively trains language models to act as optimal text compressors, directly linking compression efficiency to intelligence.

### qwen3:4b-instruct

82.5 s · 169 output tokens

The video explains how cross-entropy, a concept from data compression, is fundamentally connected to how modern language models are trained. It shows that cross-entropy measures how well a compression scheme optimized for one distribution performs on another, and this same formula appears in language model training as a loss function that minimizes the average information per token. The core insight is that training a language model using cross-entropy loss is equivalent to training it to be the best possible text compressor, supporting the idea that "compression is intelligence." The video illustrates this through examples like language tree clustering using file compression and distillation, where a small model learns from a larger one's probability distributions. A key mathematical property of cross-entropy—its minimum value when two distributions match—underpins both compression efficiency and model performance, linking the theory of compression to machine learning.

## browser tabs

`S2026E100601` · es · 7 min

### gemma3:4b

23.0 s · 148 output tokens

Este video presenta una utilidad de línea de comandos llamada “browser tabs CLI” diseñada para automatizar la gestión de pestañas en navegadores, específicamente utilizando Vivaldi. El creador desarrolló esta herramienta para optimizar el proceso de preparación de directos semanales (“cafés”) con Codely, donde se discuten noticias y se manipulan enlaces. La herramienta permite abrir grupos de pestañas desde un archivo JSON, renombrar las pestañas y personalizar su apariencia, automatizando así tareas repetitivas y ahorrando tiempo.  Finalmente, el proyecto es de código abierto y busca ser una solución accesible para la gestión de pestañas, con el potencial de adaptarse a otros navegadores y simplificar el proceso de release de software.

### qwen3:4b (thinking)

72.7 s · 1075 output tokens

El creador desarrolló una utilidad de CLI llamada browser tabs para gestionar pestañas en Vivaldi, agrupando enlaces de noticias y recursos en categorías para facilitar la navegación durante los cafés semanales de Codely. La herramienta, open source y compatible con la API de Vivaldi, permite automatizar la organización de pestañas mediante archivos JSON, asignando nombres y colores personalizados. Además, integra el sistema de releases semánticos para simplificar las actualizaciones en proyectos y cuenta con licencia MIT, evitando costos operativos al reducir el tiempo semanal dedicado a gestionar enlaces. Su enfoque práctico y adaptable garantiza utilidad para equipos que requieran gestión eficiente de navegación.

### qwen3:4b-instruct

37.7 s · 217 output tokens

El video presenta una herramienta de línea de comandos llamada browser tabs CLI, diseñada para gestionar pestañas en navegadores como Vivaldi, con funcionalidades como abrir grupos de enlaces en batch, renombrar pestañas y personalizar colores y temas. La herramienta se basa en la API directa de Vivaldi y está disponible en GitHub y NPM, con un enfoque open source que permite adaptaciones futuras para otros navegadores. El creador explica cómo automatiza la preparación de sus directos semanales en Codely, ahorrando tiempo al organizar noticias y enlaces de forma eficiente. Además, incluye prácticas de desarrollo como integración continua, gestión automática de versiones mediante semantic release y un diseño amigable con temas oscuros y claros. La herramienta, aunque enfocada en un nicho específico, se valora como una solución útil y sencilla que puede ser aprovechada por otros usuarios o proyectos.

## La Aparente Inutilidad de la IA Cuántica

`S2026E100801` · es · 21 min

### gemma3:4b

44.6 s · 146 output tokens

Este video explora la tecnología emergente de la inteligencia artificial cuántica, presentando su potencial y sus posibles desafíos. El video explica que la IA cuántica se basa en el uso de qubits, que pueden existir en múltiples estados a la vez, para resolver problemas complejos que son intratables para las computadoras clásicas.  Se ilustra cómo se podría aplicar este concepto, por ejemplo, para clasificar datos de manera más eficiente, aunque se reconoce que la tecnología aún está en sus primeras etapas y su aplicación práctica es limitada debido a la complejidad y los costos asociados.  Finalmente, el video sugiere que la IA cuántica podría complementar la IA clásica en áreas específicas, impulsando la innovación en campos como la ciencia avanzada.

### qwen3:4b (thinking)

151.2 s · 1625 output tokens

El video examina la inteligencia artificial cuántica, destacando que aunque empresas como IBM y Google la desarrollan, su aplicación práctica es limitada por la inexistencia de ordenadores cuánticos suficientemente avanzados. Un ejemplo teórico muestra cómo podría resolver problemas estructurados, como clasificar datos con patrones específicos, pero no se traduce en beneficios en tareas cotidianas como diagnósticos médicos. Los investigadores concluyen que, en el corto plazo, la IA cuántica solo complementará a la inteligencia artificial clásica en áreas científicas especializadas, sin reemplazarla en el ámbito general.

### qwen3:4b-instruct

68.0 s · 203 output tokens

El video explica que la inteligencia artificial cuántica es una tecnología teórica y aún en desarrollo, con aplicaciones potenciales en problemas muy específicos como la clasificación de datos organizados en patrones repetitivos. Se muestra cómo, en ciertos escenarios, una IA cuántica puede resolver problemas que son inabordables para los ordenadores clásicos, aunque estos casos son artificiales y no tienen aplicaciones prácticas en medicina o tecnología cotidiana. Aunque existen ventajas teóricas, los costos de implementación, los errores en los qubits y la necesidad de ordenadores cuánticos avanzados limitan su uso actual. La conclusión es que la IA cuántica no reemplazará a la IA clásica, sino que podría complementarla en tareas de ciencia avanzada, impulsando así el desarrollo de algoritmos tanto clásicos como cuánticos.

## How to Deal With the e4 Knight in the Stonewall

`S2026E101201` · en · 28 min

### gemma3:4b

44.8 s · 126 output tokens

This YouTube video unpacks a social media interaction sparked by a chess position featuring the “E4 knight” against the Stonewall structure. The video’s main point is to illustrate how easily a position can go wrong with moves like taking on D5 or placing a bishop on G5, leading to a weakened position. The presenter demonstrates the correct approach – avoiding immediate trades and maintaining tension – using examples of good and bad moves, ultimately highlighting the importance of strategic positional awareness and understanding the nuances of the Stonewall setup. Ultimately, the video teaches viewers to prioritize controlling the center and avoiding unnecessary exchanges to secure a favorable outcome.

### qwen3:4b (thinking)

96.8 s · 663 output tokens

The video analyzes how to handle a knight on e4 in the Stonewall opening structure. The creator identifies two critical mistakes: capturing on d5, which weakens center control, and placing a bishop on g5, which becomes trapped after white plays g3. The solution involves avoiding d5 captures entirely and using f3 to either kick the e4 knight out or trade into it before challenging the position. This approach preserves tension and leads to strategic advantages by preventing the knight from trading off while maintaining active play on the queenside.

### qwen3:4b-instruct

63.9 s · 169 output tokens

The video analyzes how to handle a knight on e4 in the Stonewall structure, emphasizing that taking on d5 or playing bishop g5 are serious positional mistakes that weaken white's position. It concludes that the correct approach is to avoid trading center, keep tension on the center, and instead prepare to kick the e4 knight with f3, either by first trading into it or by removing pieces from its scope. The main lesson is that white should never voluntarily release tension, and instead build a strong, coordinated attack using knight maneuvers and queen-side pressure. Through multiple game examples, the video shows how proper development and strategic planning allow white to dominate the position and eventually win. The key takeaway is that the knight on e4 is not a problem piece—when handled correctly, it becomes a powerful tool in a well-structured attack.

## I Tried the Accelerated Dragon... Not What I Expected

`S2026E102001` · en · 58 min

### gemma3:4b

59.5 s · 99 output tokens

This video documents a game between black and white played with the accelerated dragon opening. Black initially aimed for a complex position with moves like knight C6 and bishop B5, attempting to create an advantage through a pawn structure and active pieces. However, White responded with strategic moves like knight D3 and rook E1, leading to a more tactical and ultimately equal endgame. Despite a few promising moments, Black struggled to convert their initial setup into a decisive advantage, concluding the game with a draw.

### qwen3:4b (thinking)

129.5 s · 727 output tokens

The creator analyzed an endgame after playing the Accelerated Dragon opening with black pieces, where they achieved temporary advantage through king maneuvering but ultimately drew the game. They later discussed handling isolated pawn structures in Alapin openings by prioritizing piece activity over immediate attacks. Despite missing a winning move in the endgame, the video highlighted how technical play can lead to draws while avoiding aggressive pitfalls. The creator concluded that practical implementation of the opening often favors positional stability over theoretical complexity.

### qwen3:4b-instruct

98.3 s · 190 output tokens

> ⚠️ Describes one game that the player won (game 2). The video has three games: two wins, then a draw. All three models summarized it as a single game.

The video documents a chess game where the player attempts the accelerated dragon opening, starting with knight C3 and aiming for a developed, active position. Despite initial plans to punish the opponent's central pawn structure with moves like D5 and knight D5, the player later realizes that a more solid central pawn structure with E5 would have been better, leading to a more balanced and safer position. The game evolves into a tense endgame where the player uses positional pressure, king activity, and rook maneuvering to gradually gain an advantage, ultimately leading to a win after a series of missed opportunities and passive responses from the opponent. The player acknowledges that while the opening phase had a few missteps, the endgame showed a clear understanding of piece coordination and pressure. The overall conclusion is that the accelerated dragon, especially with a double fianchetto and isolated pawn structures, can lead to dynamic and instructive positions, even if not always immediately winning.

## Claude Duet review- Great design, but...

`S2026E102101` · en · 16 min

### gemma3:4b

35.1 s · 130 output tokens

This YouTube review explores Anthropic’s new personal agent, Claude Duet, which is praised for its exceptionally well-designed interface and polished user experience. Despite its attractive design and compatibility with existing Claude plugins, the reviewer found Duet lacking in proactive engagement and speed, struggling to compete with agents like Muse for driving consistent task completion. The agent’s inability to create a launch video and its disconnected experience from the desktop app ultimately led the reviewer to believe it wasn’t ready to replace existing agents in their workflow. While aesthetically pleasing, Duet needs to be more assertive and faster to truly capture user attention and become a reliable daily tool.

### qwen3:4b (thinking)

77.2 s · 601 output tokens

The video reviews Anthropic's Claude Duet personal agent, highlighting its exceptionally polished design with intuitive onboarding, task lists, artifacts, and channel-based workflows that feel more refined than competitors like OpenAI's Dots. Despite its elegant interface and strong foundational features, the reviewer finds it lacks sufficient proactivity—failing to push timely notifications—and suffers from slow performance that frustrates task completion, especially for mobile users. The agent also lacks a desktop app and doesn't support seamless multiplayer collaboration, making it less practical for small business teams despite its solid single-user capabilities. While Claude Duet is significantly more polished than early versions, the reviewer concludes it needs better urgency and speed to compete for attention in the crowded personal agent space.

### qwen3:4b-instruct

44.2 s · 158 output tokens

The video reviews Anthropic's new personal agent, Claude Duet, praising its elegant, well-designed interface and thoughtful features like task lists, artifacts, and channels that mimic Slack. While the app is polished and functional, the reviewer finds it lacking in proactivity, speed, and urgency—its slow performance and minimal notifications make it hard to re-engage with over time. Duet fails to integrate local skills and lacks multiplayer collaboration, limiting its usefulness for small business owners who need to share work with teams. Despite strong foundational design and model intelligence, the agent doesn't deliver enough real-time value or workflow closure to compete with other personal agents. The reviewer concludes that Duet is promising but needs significant improvements in responsiveness, proactive engagement, and team collaboration to become a viable daily tool.

## I Built My Own Claude Code (To Learn)

`S2026E102201` · es · 13 min

### gemma3:4b

38.8 s · 177 output tokens

Este video explica la creación de un agente de inteligencia artificial similar a Cloud Code, diseñado para aprender y entender el funcionamiento de los "harneses" de IA. El creador desarrolló un proyecto en Go utilizando la API de Anthropic, enfocándose en el bucle del agente, la construcción del contexto y la implementación de herramientas como la llamada a archivos.  El objetivo principal es desmitificar el "arnés de IA" demostrando que no se trata solo de prompts, sino de un bucle continuo de interacción con un modelo de lenguaje.  Además, el video introduce la posibilidad de expandir el proyecto, incluyendo funcionalidades como la orquestación de múltiples agentes y la visualización de su estado, como se ve en aplicaciones como Orca.  Finalmente, el creador invita a los espectadores a explorar el repositorio y a compartir ideas para futuras implementaciones.

### qwen3:4b (thinking)

150.2 s · 1500 output tokens

El creador desarrolló un agente de inteligencia artificial en Go utilizando la API de Antropic para comprender cómo funcionan los bucles de agentes, la gestión de herramientas y la memoria. Amplió el proyecto implementando subagentes que pueden ejecutar tareas como crear archivos y monitorear su progreso en tiempo real. La explicación concluye que los arneses de IA son simplemente wrappers alrededor de modelos de lenguaje, destacando la importancia de la práctica gradual para desmitificar herramientas complejas.

### qwen3:4b-instruct

56.7 s · 214 output tokens

El video explica cómo funciona el bucle agéntico y el bucle REPL en un arnés de inteligencia artificial, mostrando cómo un agente llama a una LLM, ejecuta herramientas y maneja múltiples subagentes en paralelo. El creador implementa un sistema que permite espawnear subagentes para tareas como crear ficheros, con seguimiento en tiempo real de su estado y actividad. A través de un repositorio y una página web, se ofrece un tutorial paso a paso para entender y construir arneses de IA de forma práctica. Se destaca que un agente no es algo complejo, sino un código que envuelve llamadas a una LLM, y que se puede extender con funcionalidades como límites de tiempo, selección de proveedores o memoria en la nube. El enfoque principal es desmitificar estas herramientas al construirlos desde cero, aprendiendo por la práctica y no solo por teoría.

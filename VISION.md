# Wagner Vision

## One Line

Wagner is a local-first, cloud-connected personal operating system for daily work:
one place to talk, type, search, run agents, build workflows, manage knowledge,
use connected tools, and do focused work in dedicated workspaces.

## Product Thesis

Wagner should feel like a serious smart system for one person's work. The closest
reference is "Maven-like" in the Palantir smart-system sense: one operating
picture, typed knowledge, source-aware analysis, and coordinated action. It is
not an open-source intelligence product, a military system, or a surveillance
console. It is a personal OS for daily work.

The starting user is Mark: a software engineer and founder who wants to run his
daily operations through one system instead of switching between coding tools,
chat apps, knowledge tools, search, email, docs, and workflow runners. Coding and
development are central, but they are not the whole product. Coding is the first
heavy workspace inside a broader system that can grow into research, briefings,
media generation, productivity automation, and team knowledge.

## Jobs To Do

Wagner should be able to:

- Capture intent by voice or text and turn it into action.
- Run agentic work through tools such as Codex, Claude Code, Claude Agent SDK,
  and future harnesses that are valuable to the operator.
- Run deterministic workflows as well as agentic workflows: scheduled jobs,
  repeatable automations, human-in-the-loop procedures, and headless tasks.
- Maintain a local-first vault knowledge graph that can sync across devices and
  teams when explicitly enabled.
- Connect to productivity systems such as Slack, Discord, Jira, Notion, email,
  GitLab, calendars, docs, and browsers through MCP or equivalent connectors.
- Act as a general search and research tool: web search, news lookup, source
  correlation, documents, links, images, and concise AI analysis.
- Generate and manipulate artifacts when useful, including images, reports,
  documents, briefings, workflows, notes, and code.
- Render results in a useful UI rather than dumping text: daily news briefings,
  search result collections, image grids, document/link sets, workflow status,
  coding sessions, and knowledge graph views.

## Pillars

### 1. Voice-First, Text-Equal Assistant

Voice is a first-class interface, not a novelty toggle. The operator should be
able to say what they want, have Wagner capture intent, dispatch agents or
workflows, report progress, and speak or show the result. Text remains fully
equal: every voice action must be possible by typing or clicking. The assistant
identity is customizable and should communicate in explicit, low-metaphor,
engineer-grade language.

### 2. Agents, Skills, Models, And Harnesses

Agents are user-authored and configurable: name, system prompt, model, tools,
allowed directories, skills, and harness. Wagner should lean into the tools the
operator already values, including Codex and Claude Code, while leaving room for
Claude Agent SDK, OpenAI/other provider SDKs, Cursor/OpenCode-style harnesses,
local models, and automatic model routing. Skills should be portable across
harnesses wherever possible.

### 3. Workflows

Wagner is not only conversational agents. It should support deterministic and
agentic workflows in the same operating picture: "every Friday at 2pm, read Jira
epics and recent completions, prepare a status page, and post it to Slack" is a
first-class product target. Workflows can be authored by voice, text, or a visual
builder, and their outputs should land back in the vault when useful.

### 4. Local-First Vault Knowledge Graph

The vault is the memory layer. It should be local-first and useful offline, with
cloud sync available for the operator's other devices or for teams. Knowledge is
stored as curated notes and typed relationships, not as raw transcript dumps. The
graph should support retrieval, provenance, links between agents/workflows/tools,
and source-aware summaries. Sync must respect the privacy boundary: default sync
is curated knowledge and metadata, not raw code, secrets, private files, or full
agent transcripts.

### 5. Connectors, Search, And Research

Wagner should reach the operator's real work surface: Slack, Discord, Jira,
Notion, email, GitLab, calendars, docs, browsers, and search providers. It should
serve as a general search tool as much as a coding assistant: look up news, search
the web, correlate sources, find documents and images, and present the result as
structured UI with links, source context, and an AI breakdown.

### 6. Dedicated Workspaces

The home surface is a general operating picture over all work. Heavy work earns a
dedicated workspace when it needs specialized tools. Coding is the first heavy
workspace: repos, files, diffs, sessions, permissions, tests, logs, and agent
activity. Later workspaces may cover research, workflow building, media creation,
knowledge editing, connector administration, or other high-focus domains.

## Experience Principles

- **Personal OS, not narrow app.** Do not frame Wagner as a coding IDE, a chat
  bot, or a status dashboard. It is an extensible work system.
- **Voice-first, text-equal.** Voice should feel native; text and click paths
  must remain complete.
- **Local-first by default.** The operator's machine is the primary execution and
  knowledge environment. Cloud extends, syncs, and coordinates; it should not be
  required for local work.
- **Smart-system clarity.** The UI should expose entities, relationships,
  provenance, status, and next actions without jargon.
- **Cinematic, grounded, precise.** The product can feel futuristic and alive,
  but labels and copy must stay plain English.
- **Bespoke, not template.** Avoid generic AI-app tells: empty card grids,
  gradient hero text, cream/sand defaults, vague dashboards, and decorative
  sci-fi language that hides meaning.

## Non-Goals

- Wagner is not an OSINT tool, military/intelligence product, or surveillance
  feed.
- Wagner is not only a coding tool or Cursor clone.
- Wagner is not a passive voice bot; it is a workbench where real work happens.
- Wagner is not a cloud control plane that owns all execution. Local execution
  and local knowledge remain first-class.

## Design-System Blurb

Wagner: a local-first, voice-first personal OS for daily work. It combines
custom agents, Codex/Claude-style coding work, deterministic and agentic
workflows, productivity connectors, general search and news research, media and
artifact generation, and a cloud-connected vault knowledge graph into one
operating picture. It should feel like a serious smart system for one person's
work: cinematic, grounded, precise, extensible, bespoke, and useful for anything
from coding to research to daily briefings.

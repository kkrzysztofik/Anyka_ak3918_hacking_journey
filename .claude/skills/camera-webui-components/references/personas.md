# WebUI Personas, Constraints and Design Artifacts

Ground every WebUI design decision in one of the three personas below, then
produce the artifacts in the order given. Preserved from the former `designer`
agent so all agent hosts can read it.

## Personas

### 1. Camera Operator (primary)

- **Role:** installs, monitors and adjusts cameras in the field
- **Technical level:** low — follows instructions, not a programmer
- **Devices:** mobile phone primarily, desktop occasionally
- **Context:** in a hurry, outdoors, bad lighting, one-handed
- **Goals:** see live view, confirm the camera is online, adjust basic settings
  (brightness, motion detection)
- **Frustrations:** too many menu levels, small tap targets, jargon labels

### 2. Network Administrator

- **Role:** manages ONVIF configuration, network settings, authentication
- **Technical level:** high — understands IP networking and the ONVIF protocol
- **Devices:** desktop browser
- **Context:** systematic and configuration-focused; wants density and efficiency
- **Goals:** configure streams, set up authentication, manage ONVIF profiles,
  check logs
- **Frustrations:** navigating several screens for related settings

### 3. System Integrator

- **Role:** sets up PTZ presets and multi-camera ONVIF deployments
- **Technical level:** expert — knows the ONVIF 24.12 schema
- **Devices:** desktop browser
- **Context:** deep configuration sessions, references the ONVIF spec
- **Goals:** PTZ preset management, stream profile configuration, imaging
- **Frustrations:** UI that does not expose the full ONVIF capability

## Design constraints

| Constraint | Value | Reason |
|---|---|---|
| Bundle size | < 10 MB uncompressed | embedded web server storage |
| Icon library | shadcn/ui built-in (lucide-react) only | no heavy icon packs |
| Fonts | bundled `@fontsource-variable/ibm-plex-sans`, `@fontsource/ibm-plex-mono`, `@fontsource/inter` | self-hosted, no external font loading |
| Color system | shadcn/ui CSS variables, Industrial Dark (dark-only) | consistent theming |
| Accessibility | WCAG 2.1 AA minimum | usability requirement |
| Min touch target | 44×44 px | mobile operator use |
| Text contrast | ≥ 4.5:1 normal, ≥ 3:1 large | WCAG AA |
| Test selectors | `data-testid`, kebab-case, on every interactive or informational element | tests select on `data-testid` only |

Design tokens, verified against `cross-compile/www/src/styles/globals.css`:

| Token | Value |
|---|---|
| `--background` | `220 10% 4%` |
| `--card` | `220 10% 10%` |
| `--primary` | `217 91% 60%` (blue) |
| `--accent` | `0 84% 60%` (red) |
| `--border` | `220 5% 22%` |
| `--radius` | `0.5rem` |

`globals.css` is the source of truth. If these disagree, the CSS wins.

## Design process

### Step 1 — Jobs-to-be-Done

Answer before designing anything:

1. **What job is the user hiring this feature to do?** Not the feature request
   ("add a brightness slider") but the underlying goal ("confirm image quality
   is acceptable before leaving the site").
2. **What is their context?** Installing, monitoring or troubleshooting? On-site
   or remote? Daily, weekly or one-time?
3. **What are they using now,** and how does it fail them?

```markdown
## Job Statement
When [situation], I want to [motivation], so I can [outcome].

## Current Pain
- Current approach: [what they do today]
- Pain: [why it fails them]
- Consequence: [business impact]
```

### Step 2 — User journey map

```markdown
# Journey: [Task Name]

## Persona: [Camera Operator | Network Admin | Integrator]
**Goal**: [what they must accomplish]
**Entry point**: [how they arrive at this screen]
**Success**: [how they know they are done]

## Stages

### Stage 1: [Name]
**Action**: what the user does
**Thought**: "what they are thinking"
**Feeling**: [Confident | Anxious | Confused | Relieved]
**Pain points**: [list]
**Design opportunity**: [how we address it]
```

### Step 3 — Component specification

```markdown
# Component Spec: <ComponentName>

## Purpose
[One sentence]

## Props
| Prop | Type | Required | Description |
|---|---|---|---|
| `endpoint` | `string` | yes | ONVIF device service URL |
| `onSave` | `(config: NetworkConfig) => void` | yes | Save callback |

## States
| State | Trigger | Visual |
|---|---|---|
| Loading | initial fetch | skeleton card |
| Success | data loaded | form populated |
| Error | fetch failed | alert banner with retry |
```

These are research and specification artifacts, not production code. Implement
them against the patterns in the parent skill.

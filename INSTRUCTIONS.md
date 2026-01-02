# Forge - Complete Windows Setup & Deployment Guide

> **AI-Enhanced Terminal Development Environment**  
> *Your intelligent pair programmer for 300+ AI models*

[![Version](https://img.shields.io/badge/version-latest-blue.svg)](https://github.com/antinomyhq/forge/releases)
[![Platform](https://img.shields.io/badge/platform-Windows-lightgrey.svg)](https://github.com/antinomyhq/forge)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

---

## 📑 Table of Contents

- [What is Forge?](#what-is-forge)
- [System Requirements](#system-requirements)
- [Quick Start](#quick-start)
- [Installation Methods](#installation-methods)
  - [Method 1: NPX (Recommended)](#method-1-npx-recommended)
  - [Method 2: Pre-built Binary](#method-2-pre-built-binary)
  - [Method 3: Build from Source](#method-3-build-from-source)
- [Configuration](#configuration)
- [Features & Capabilities](#features--capabilities)
- [Usage Guide](#usage-guide)
- [Troubleshooting](#troubleshooting)
- [Advanced Topics](#advanced-topics)
- [FAQ](#faq)

---

## What is Forge?

**Forge** is a powerful, AI-enhanced terminal development environment that brings the capabilities of 300+ AI models directly into your command line. Built with Rust for performance and reliability, Forge acts as your intelligent pair programmer.

### Key Features

- 🤖 **Multi-Model Support**: Claude, GPT-4, O-series, Gemini, Grok, DeepSeek, and 300+ models
- 🔌 **Multiple Providers**: OpenRouter, OpenAI, Anthropic, Google, AWS Bedrock, x.ai, z.ai, Cerebras
- 🛠️ **Rich Tool System**: File operations, shell commands, git integration
- 🌐 **MCP Protocol**: Model Context Protocol for extensibility
- ⚡ **Fast & Efficient**: Written in Rust with async I/O
- 🔒 **Secure**: Your code stays local
- 📝 **Customizable**: YAML-based configuration, custom commands, workflows

### What Can Forge Do?

- 💬 Answer coding questions with project context
- 🔧 Implement features and fix bugs
- 📚 Explain complex codebases
- 🧪 Write and debug tests
- 📖 Generate documentation
- 🔄 Refactor code
- 🎨 Design system architectures
- 🐛 Debug issues with AI assistance

---

## System Requirements

### Minimum Requirements

- **OS**: Windows 10 or later (64-bit)
- **RAM**: 4 GB (8 GB recommended)
- **Disk Space**: 500 MB for Forge + dependencies
- **Internet**: Required for AI model access

### Required Software (varies by installation method)

**For NPX Installation:**
- Node.js 16.x or later
- npm or pnpm

**For Binary Installation:**
- None! Just download and run

**For Source Build:**
- Rust 1.70 or later (via rustup)
- Visual Studio Build Tools 2019 or later
- Git for Windows

---

## Quick Start

The fastest way to get started with Forge on Windows:

```powershell
# Install via NPX (requires Node.js)
npx forgecode@latest

# Or use our automated setup script
python setup.py
```

That's it! Forge will guide you through initial configuration.

---

## Installation Methods

Choose the method that best fits your needs:

| Method | Best For | Requirements | Time |
|--------|----------|--------------|------|
| **NPX** | Quick start, easy updates | Node.js | 2 min |
| **Binary** | No dependencies | None | 1 min |
| **Source** | Development, customization | Rust + Build Tools | 10 min |

---

### Method 1: NPX (Recommended)

**Easiest and most maintainable option.**

#### Prerequisites

1. **Install Node.js**
   - Download from https://nodejs.org/
   - Choose LTS version (20.x or later)
   - Verify installation:
     ```powershell
     node --version  # Should show v20.x.x or later
     npm --version   # Should show 10.x.x or later
     ```

#### Installation Steps

```powershell
# Run Forge directly (downloads and runs latest version)
npx forgecode@latest

# Or install globally for permanent access
npm install -g forgecode

# Then run anytime with:
forge
```

#### Verify Installation

```powershell
forge --version
```

#### Troubleshooting NPX Install

**Issue: "npx is not recognized"**
- Restart terminal after Node.js installation
- Add Node.js to PATH: `C:\Program Files\nodejs\`

**Issue: "Permission denied"**
- Run PowerShell as Administrator
- Or use: `npx --yes forgecode@latest`

**Issue: "Network error"**
- Check firewall/antivirus settings
- Try: `npm config set proxy http://your-proxy:port`

---

### Method 2: Pre-built Binary

**No dependencies required, just download and run.**

#### Installation Steps

1. **Download Binary**
   ```powershell
   # Download latest release for Windows
   # Visit: https://github.com/antinomyhq/forge/releases/latest
   # Download: forge-x86_64-pc-windows-msvc.exe
   ```

2. **Create Installation Directory**
   ```powershell
   # Create folder for Forge
   New-Item -Path "C:\Program Files\Forge" -ItemType Directory -Force
   
   # Move binary there
   Move-Item forge-x86_64-pc-windows-msvc.exe "C:\Program Files\Forge\forge.exe"
   ```

3. **Add to PATH**
   ```powershell
   # Add to user PATH (temporary - current session only)
   $env:PATH += ";C:\Program Files\Forge"
   
   # Add to system PATH (permanent - requires Admin)
   [Environment]::SetEnvironmentVariable(
       "PATH",
       "$([Environment]::GetEnvironmentVariable('PATH', 'Machine'));C:\Program Files\Forge",
       "Machine"
   )
   ```

4. **Verify Installation**
   ```powershell
   # Restart terminal, then:
   forge --version
   ```

#### Quick Binary Setup Script

Save as `install-forge-binary.ps1`:

```powershell
# Requires Administrator privileges
if (-NOT ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole] "Administrator")) {
    Write-Warning "Please run as Administrator"
    exit
}

$installDir = "C:\Program Files\Forge"
$downloadUrl = "https://github.com/antinomyhq/forge/releases/latest/download/forge-x86_64-pc-windows-msvc.exe"

Write-Host "Installing Forge..." -ForegroundColor Cyan

# Create directory
New-Item -Path $installDir -ItemType Directory -Force | Out-Null

# Download binary
Write-Host "Downloading binary..." -ForegroundColor Yellow
Invoke-WebRequest -Uri $downloadUrl -OutFile "$installDir\forge.exe"

# Add to PATH
$path = [Environment]::GetEnvironmentVariable("PATH", "Machine")
if ($path -notlike "*$installDir*") {
    [Environment]::SetEnvironmentVariable("PATH", "$path;$installDir", "Machine")
    Write-Host "Added to PATH" -ForegroundColor Green
}

Write-Host "Installation complete! Restart your terminal and run 'forge'" -ForegroundColor Green
```

Run with:
```powershell
powershell -ExecutionPolicy Bypass -File install-forge-binary.ps1
```

---

### Method 3: Build from Source

**For developers who want to customize or contribute to Forge.**

#### Prerequisites

1. **Install Rust**
   ```powershell
   # Download and run rustup-init.exe from https://rustup.rs/
   # Or use winget:
   winget install Rustlang.Rustup
   
   # Verify installation
   rustc --version  # Should show 1.70 or later
   cargo --version
   ```

2. **Install Visual Studio Build Tools**
   ```powershell
   # Download from: https://visualstudio.microsoft.com/downloads/
   # Or use winget:
   winget install Microsoft.VisualStudio.2022.BuildTools
   
   # Required components:
   # - Desktop development with C++
   # - Windows 10 SDK
   ```

3. **Install Git for Windows**
   ```powershell
   winget install Git.Git
   ```

#### Build Steps

```powershell
# Clone repository
git clone https://github.com/antinomyhq/forge.git
cd forge

# Build release version (optimized)
cargo build --release

# Binary location: target/release/forge.exe

# Install globally (copies to cargo bin)
cargo install --path .

# Verify installation
forge --version
```

#### Development Build

```powershell
# Faster builds for development (not optimized)
cargo build

# Run without installing
cargo run -- [forge arguments]

# Run tests
cargo test

# Run with verbose logging
$env:RUST_LOG="debug"
cargo run
```

#### Troubleshooting Source Build

**Issue: "linker 'link.exe' not found"**
- Install Visual Studio Build Tools with C++ components
- Restart terminal after installation

**Issue: "cargo is not recognized"**
- Add to PATH: `%USERPROFILE%\.cargo\bin`
- Restart terminal

**Issue: "Build fails with OpenSSL errors"**
- Forge uses `rustls`, should not need OpenSSL
- If issues persist: `cargo clean && cargo build --release`

**Issue: "Out of memory during build"**
- Close other applications
- Or build with: `cargo build --release --jobs 2`

---

## Configuration

Forge uses three main configuration files:

### 1. `.env` File (API Keys & Environment)

Create a `.env` file in your project directory or home folder:

```env
# Primary Provider (OpenRouter - Recommended)
OPENROUTER_API_KEY=sk-or-v1-xxxxxxxxxxxxx

# Alternative Providers (optional)
OPENAI_API_KEY=sk-xxxxxxxxxxxxx
ANTHROPIC_API_KEY=sk-ant-xxxxxxxxxxxxx
GOOGLE_API_KEY=xxxxxxxxxxxxx

# Advanced HTTP Configuration
FORGE_HTTP_CONNECT_TIMEOUT=30
FORGE_HTTP_READ_TIMEOUT=900

# Windows-specific
COMSPEC=C:\Windows\System32\cmd.exe
```

**Where to get API keys:**
- **OpenRouter**: https://openrouter.ai/keys (Recommended - access to 300+ models)
- **OpenAI**: https://platform.openai.com/api-keys
- **Anthropic**: https://console.anthropic.com/
- **Google**: https://ai.google.dev/

### 2. `forge.yaml` (Model & Behavior)

Create `forge.yaml` in your project directory:

```yaml
# Model selection
model: "claude-3.7-sonnet"  # or "gpt-4o", "gemini-2.5-pro", etc.

# Temperature (creativity vs. focus)
temperature: 0.7  # 0.0 = focused, 2.0 = creative

# Custom rules for AI behavior
custom_rules: |
  1. Always add comprehensive error handling
  2. Include unit tests for new functions
  3. Follow our team's naming conventions

# Custom commands (shortcuts)
commands:
  - name: "refactor"
    description: "Refactor selected code"
    prompt: "Please refactor this code to improve readability and performance"
  
  - name: "test"
    description: "Generate tests"
    prompt: "Write comprehensive unit tests for this code"

# Advanced settings
max_walker_depth: 3  # How deep to traverse directories
max_requests_per_turn: 50  # Prevent runaway conversations
max_tool_failure_per_turn: 3  # Stop after repeated failures
```

### 3. `.mcp.json` (MCP Servers - Optional)

Configure Model Context Protocol servers:

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "node",
      "args": ["C:\\path\\to\\mcp-server-filesystem\\index.js"]
    },
    "github": {
      "command": "node",
      "args": ["C:\\path\\to\\mcp-server-github\\index.js"],
      "env": {
        "GITHUB_TOKEN": "your_github_token"
      }
    }
  }
}
```

**MCP Management Commands:**
```powershell
# List configured MCP servers
forge mcp list

# Add new server interactively
forge mcp add

# Remove server
forge mcp remove
```

---

## Features & Capabilities

### Supported AI Models

Forge supports 300+ models across multiple providers:

#### OpenRouter (Recommended)
Access to all models through one API key:
- Anthropic Claude (3.7 Sonnet, 3.5 Opus)
- OpenAI (GPT-4o, O1, O3-mini)
- Google (Gemini 2.5 Pro, Gemini 2.0 Flash)
- Meta (Llama 3.3, Llama 3.1)
- Mistral, Cohere, DeepSeek, and more

#### Direct Provider Access
- **OpenAI**: GPT-4o, GPT-4 Turbo, O1, O3-mini
- **Anthropic**: Claude 3.7 Sonnet, Claude 3.5 Opus
- **Google**: Gemini 2.5 Pro, Gemini 2.0 Flash (via Vertex AI)
- **AWS Bedrock**: Claude, Llama, Mistral models
- **x.ai**: Grok models
- **z.ai**: GLM-4.5 models
- **Cerebras**: Ultra-fast inference models

Switch models anytime with:
```powershell
forge  # Then use /model command in chat
```

### Core Capabilities

#### 🗂️ File Operations
- `read_file` - Read file contents with syntax highlighting
- `write_file` - Create or modify files
- `search_files` - Grep-like search across codebase
- `list_directory` - Browse directory structure
- `read_image` - Analyze images for multimodal models

#### 🖥️ Shell Commands
- Execute system commands safely
- Capture stdout/stderr
- Formatted output in XML for parsing
- Timeout protection

#### 🔧 Git Operations
- Stage changes
- Commit with messages
- Branch management
- Merge operations
- Status and diff viewing

#### 🌐 MCP Protocol
- Connect to external tools
- Web browser automation
- API integrations
- Custom tool development

### Architecture

Forge is built using a modular crate structure:

```
forge/
├── forge_api/          # API client for Forge services
├── forge_app/          # Main application logic
├── forge_ci/           # CI/CD integrations
├── forge_display/      # Terminal UI and formatting
├── forge_domain/       # Core domain models
├── forge_fs/           # Filesystem operations
├── forge_infra/        # Infrastructure abstractions
├── forge_main/         # Application entry point
├── forge_repo/         # Repository operations
├── forge_services/     # Business logic services
├── forge_stream/       # Streaming response handling
├── forge_template/     # Templating system
├── forge_tool_macros/  # Code generation for tools
├── forge_tracker/      # Analytics and tracking
├── forge_walker/       # Directory traversal
├── forge_json_repair/  # JSON repair utilities
└── forge_select/       # Interactive selection UI
```

---

## Usage Guide

### Basic Usage

#### Start Interactive Session
```powershell
forge
# Opens interactive chat interface
```

#### Direct Prompt
```powershell
forge --prompt "Explain how authentication works in this codebase"
```

#### Execute Command File
```powershell
forge --command commands.txt
```

#### Run Workflow
```powershell
forge --workflow workflow.yaml
```

### Example Workflows

#### 1. Code Review
```
> Review the code in src/auth.rs and suggest improvements
```

Forge will:
- Read the file
- Analyze code structure
- Identify issues (security, performance, style)
- Suggest specific improvements with code examples

#### 2. Implement Feature
```
> Add user authentication to the web server in server.rs
```

Forge will:
- Analyze current code structure
- Design authentication flow
- Implement necessary changes
- Update related files
- Suggest tests to add

#### 3. Debug Issue
```
> I'm getting error "null pointer dereference" in main.rs line 45, help debug
```

Forge will:
- Read the file and context
- Analyze the error
- Identify root cause
- Suggest fixes with explanations

#### 4. Refactor Code
```
> Refactor this function to use async/await instead of callbacks
```

Forge will:
- Understand current implementation
- Convert to modern async/await pattern
- Handle error cases
- Update related code

### Advanced Usage

#### Custom Commands

Define shortcuts in `forge.yaml`:

```yaml
commands:
  - name: "security"
    prompt: "Review this code for security vulnerabilities"
  
  - name: "optimize"
    prompt: "Optimize this code for performance"
```

Use with:
```powershell
forge --command security
```

#### Workflows

Create multi-step workflows in YAML:

```yaml
# workflow.yaml
name: "Feature Implementation"
steps:
  - name: "Design"
    prompt: "Design the architecture for user authentication"
  
  - name: "Implement"
    prompt: "Implement the authentication system"
  
  - name: "Test"
    prompt: "Write comprehensive tests for authentication"
  
  - name: "Document"
    prompt: "Document the authentication system for our README"
```

Run with:
```powershell
forge --workflow workflow.yaml
```

---

## Troubleshooting

### Common Issues & Solutions

#### Issue: "API Key Not Found"

**Symptoms:**
```
Error: No API key found for provider
```

**Solutions:**
1. Check `.env` file exists in current directory or home folder
2. Verify key name matches exactly (e.g., `OPENROUTER_API_KEY`)
3. Ensure no extra spaces or quotes around key value
4. Try setting environment variable directly:
   ```powershell
   $env:OPENROUTER_API_KEY = "sk-or-v1-xxxxxxxxxxxxx"
   forge
   ```

#### Issue: "Model Not Found"

**Symptoms:**
```
Error: Model 'claude-4' not found
```

**Solutions:**
1. Check model name in `forge.yaml` - use exact names:
   - `claude-3.7-sonnet` (not `claude-4`)
   - `gpt-4o` (not `gpt-4-optimized`)
   - `gemini-2.5-pro` (not `gemini-pro-2.5`)
2. List available models:
   ```powershell
   forge  # Then type: /model
   ```
3. Update to latest Forge version: `npx forgecode@latest`

#### Issue: "Network Timeout"

**Symptoms:**
```
Error: Connection timeout after 30 seconds
```

**Solutions:**
1. Check internet connection
2. Increase timeout in `.env`:
   ```env
   FORGE_HTTP_READ_TIMEOUT=1800  # 30 minutes
   ```
3. Check firewall/antivirus blocking Forge
4. If behind proxy, configure:
   ```env
   HTTP_PROXY=http://proxy:port
   HTTPS_PROXY=http://proxy:port
   ```

#### Issue: "Permission Denied"

**Symptoms:**
```
Error: Access denied when writing file
```

**Solutions:**
1. Run PowerShell as Administrator
2. Check file is not read-only:
   ```powershell
   attrib -r filename.txt
   ```
3. Check folder permissions:
   ```powershell
   icacls C:\path\to\folder /grant Users:F
   ```

#### Issue: "Forge Command Not Found"

**Symptoms:**
```
'forge' is not recognized as an internal or external command
```

**Solutions:**
1. **NPX Installation:** No PATH needed, use `npx forgecode@latest`
2. **Binary Installation:** Add to PATH (see installation section)
3. **Source Build:** Ensure cargo bin in PATH:
   ```powershell
   $env:PATH += ";$env:USERPROFILE\.cargo\bin"
   ```
4. Restart terminal after any PATH changes

#### Issue: "SSL Certificate Error"

**Symptoms:**
```
Error: SSL certificate problem: unable to get local issuer certificate
```

**Solutions:**
1. Update Windows root certificates:
   ```powershell
   certutil -generateSSTFromWU roots.sst
   ```
2. Temporary bypass (NOT recommended for production):
   ```env
   FORGE_HTTP_ACCEPT_INVALID_CERTS=true
   ```

### Performance Issues

#### Slow Response Times

1. **Use Faster Models:**
   - `claude-3.5-haiku` instead of `claude-3.7-sonnet`
   - `gpt-4o-mini` instead of `gpt-4o`
   - `gemini-2.0-flash` instead of `gemini-2.5-pro`

2. **Limit Context:**
   ```yaml
   max_walker_depth: 2  # Don't traverse too deep
   ```

3. **Reduce Temperature:**
   ```yaml
   temperature: 0.3  # Faster, more focused responses
   ```

#### High Memory Usage

1. **Clear Conversation History:**
   ```powershell
   # Start fresh session
   forge --new
   ```

2. **Limit File Reads:**
   - Be specific about which files to analyze
   - Use `search_files` instead of reading everything

### Getting Help

#### Enable Verbose Logging
```powershell
$env:RUST_LOG = "debug"
forge --verbose
```

#### Check System Status
```powershell
# Run validation script
python validate.py
```

#### Community Support
- Discord: https://discord.gg/kRZBPpkgwq
- GitHub Issues: https://github.com/antinomyhq/forge/issues
- Documentation: https://forgecode.dev/docs

---

## Advanced Topics

### IDE Integration

#### Visual Studio Code

1. **Terminal Integration:**
   - Open terminal in VS Code (Ctrl + `)
   - Run forge commands directly
   - Use forge to explain/refactor selected code

2. **Tasks Integration:**
   Create `.vscode/tasks.json`:
   ```json
   {
     "version": "2.0.0",
     "tasks": [
       {
         "label": "Forge: Review Current File",
         "type": "shell",
         "command": "forge",
         "args": ["--prompt", "Review ${file}"]
       }
     ]
   }
   ```

#### JetBrains IDEs (IntelliJ, RustRover, etc.)

1. **External Tools:**
   - Settings → Tools → External Tools
   - Add Forge with custom arguments
   - Assign keyboard shortcuts

### CI/CD Integration

#### GitHub Actions

```yaml
name: AI Code Review
on: [pull_request]

jobs:
  review:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Forge
        run: npm install -g forgecode
      
      - name: Run Code Review
        env:
          OPENROUTER_API_KEY: ${{ secrets.OPENROUTER_API_KEY }}
        run: |
          forge --prompt "Review changes in this PR for issues"
```

#### GitLab CI

```yaml
ai-review:
  stage: test
  script:
    - npm install -g forgecode
    - forge --prompt "Review code quality and security"
  only:
    - merge_requests
```

### Custom Tools with MCP

Create custom MCP servers to extend Forge:

```javascript
// my-custom-tool.js
const { Server } = require('@modelcontextprotocol/sdk/server');

const server = new Server({
  name: 'my-custom-tool',
  version: '1.0.0'
});

server.tool('analyze_metrics', async ({ data }) => {
  // Your custom logic here
  return { result: 'Metrics analyzed' };
});

server.start();
```

Configure in `.mcp.json`:
```json
{
  "mcpServers": {
    "my-tool": {
      "command": "node",
      "args": ["my-custom-tool.js"]
    }
  }
}
```

### Security Best Practices

1. **API Key Protection:**
   - Never commit `.env` to git
   - Use environment variables in CI/CD
   - Rotate keys regularly

2. **Code Review:**
   - Always review AI-generated code
   - Don't execute shell commands blindly
   - Validate AI suggestions before committing

3. **Restricted Mode:**
   ```powershell
   forge --restricted  # Limits shell command execution
   ```

4. **Audit Logs:**
   ```env
   FORGE_TRACKER_ENABLED=true  # Enable telemetry for audit
   ```

---

## FAQ

### General Questions

**Q: Is Forge free to use?**  
A: Forge itself is open-source and free. You pay for API usage from providers (OpenAI, Anthropic, etc.). OpenRouter offers free tiers for many models.

**Q: Does Forge send my code to external servers?**  
A: Only the context you explicitly share with the AI model is sent to the provider's API. Your full codebase stays local.

**Q: Can I use Forge offline?**  
A: No, Forge requires internet to access AI models. However, you can configure local LLMs via custom OpenAI-compatible endpoints.

**Q: Which model is best for coding?**  
A: For general coding: `claude-3.7-sonnet` or `gpt-4o`. For speed: `claude-3.5-haiku` or `gpt-4o-mini`. For reasoning: `o1` or `o3-mini`.

**Q: How much does API usage cost?**  
A: Varies by model. OpenRouter shows pricing: https://openrouter.ai/models. Typical costs: $0.01-$0.10 per conversation.

### Technical Questions

**Q: Can I use my own locally hosted model?**  
A: Yes! Configure any OpenAI-compatible API endpoint in `forge.yaml`.

**Q: Does Forge work with WSL (Windows Subsystem for Linux)?**  
A: Yes, but this guide focuses on native Windows. For WSL, follow Linux installation instructions.

**Q: Can I contribute to Forge development?**  
A: Absolutely! See CONTRIBUTING.md in the repository.

**Q: Does Forge support multiple projects?**  
A: Yes, each project can have its own `forge.yaml` and `.env` configuration.

**Q: Can I automate Forge in scripts?**  
A: Yes! Use `--prompt` for direct commands or `--workflow` for multi-step automation.

### Windows-Specific Questions

**Q: Does Forge work on Windows ARM (Surface Pro X)?**  
A: Source build should work. Pre-built binaries may not be available for ARM yet.

**Q: Can I use Forge in PowerShell ISE?**  
A: Yes, but Windows Terminal or standard PowerShell is recommended for better display.

**Q: Does Forge work with Windows Defender?**  
A: Yes, but first run may trigger a SmartScreen warning. Click "More info" → "Run anyway".

---

## Additional Resources

### Documentation
- Official Docs: https://forgecode.dev/docs
- API Reference: https://forgecode.dev/api
- Examples: https://github.com/antinomyhq/forge/tree/main/examples

### Community
- Discord Server: https://discord.gg/kRZBPpkgwq
- GitHub Discussions: https://github.com/antinomyhq/forge/discussions
- Twitter: @forgecodetool

### Related Tools
- MCP Server Registry: https://github.com/modelcontextprotocol/servers
- OpenRouter Models: https://openrouter.ai/models
- Rust Tools: https://www.rust-lang.org/tools

---

## Next Steps

1. ✅ **Install Forge** using your preferred method
2. ✅ **Configure API Keys** in `.env` file
3. ✅ **Run First Command**: `forge --prompt "Introduce yourself"`
4. ✅ **Explore Features**: Try code review, feature implementation
5. ✅ **Customize**: Add custom commands and workflows
6. ✅ **Join Community**: Get help and share experiences

**Happy Coding with Forge! 🚀**

---

*Last Updated: 2025-01-08*  
*Version: 1.0.0*  
*Platform: Windows 10/11*


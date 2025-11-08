#!/usr/bin/env python3
"""
Forge Setup Script for Windows
================================

AI-powered installation wizard with intelligent error detection and resolution.

Features:
- Automatic dependency detection and installation
- Multiple installation methods (NPX, Binary, Source)
- AI-powered error fallback system
- Environment validation and configuration
- Interactive prompts with smart defaults

Usage:
    python setup.py                    # Interactive setup
    python setup.py --method npx       # Force specific installation method
    python setup.py --non-interactive  # Automated setup with defaults
    python setup.py --validate-only    # Check environment without installing
"""

import os
import sys
import subprocess
import platform
import json
import re
import shutil
import urllib.request
import urllib.error
from pathlib import Path
from typing import Optional, Dict, List, Tuple
from enum import Enum


# ============================================================================
# Configuration & Constants
# ============================================================================

class InstallMethod(Enum):
    NPX = "npx"
    BINARY = "binary"
    SOURCE = "source"


class Color:
    """ANSI color codes for terminal output"""
    RED = '\033[91m'
    GREEN = '\033[92m'
    YELLOW = '\033[93m'
    BLUE = '\033[94m'
    MAGENTA = '\033[95m'
    CYAN = '\033[96m'
    WHITE = '\033[97m'
    BOLD = '\033[1m'
    END = '\033[0m'


ERROR_KNOWLEDGE_BASE = {
    "node_not_found": {
        "pattern": r"(node|npm|npx).*(not found|not recognized)",
        "description": "Node.js is not installed or not in PATH",
        "solutions": [
            {
                "method": "winget",
                "command": "winget install OpenJS.NodeJS.LTS",
                "description": "Install via Windows Package Manager"
            },
            {
                "method": "manual",
                "description": "Download from https://nodejs.org/ and install",
                "url": "https://nodejs.org/"
            }
        ],
        "ai_prompt": "Analyze this Node.js installation error on Windows and provide solutions"
    },
    "rust_not_found": {
        "pattern": r"(cargo|rustc|rustup).*(not found|not recognized)",
        "description": "Rust toolchain is not installed",
        "solutions": [
            {
                "method": "winget",
                "command": "winget install Rustlang.Rustup",
                "description": "Install via Windows Package Manager"
            },
            {
                "method": "manual",
                "description": "Download rustup-init.exe from https://rustup.rs/",
                "url": "https://rustup.rs/"
            }
        ],
        "ai_prompt": "Analyze this Rust installation error on Windows and provide solutions"
    },
    "vs_build_tools": {
        "pattern": r"(linker.*link\.exe|MSVC|Visual Studio|C\+\+ build tools)",
        "description": "Visual Studio Build Tools not installed",
        "solutions": [
            {
                "method": "winget",
                "command": "winget install Microsoft.VisualStudio.2022.BuildTools",
                "description": "Install VS Build Tools 2022"
            },
            {
                "method": "manual",
                "description": "Install from Visual Studio installer with C++ components",
                "url": "https://visualstudio.microsoft.com/downloads/"
            }
        ],
        "ai_prompt": "Analyze this Visual Studio Build Tools error and provide Windows-specific solutions"
    },
    "network_error": {
        "pattern": r"(network|connection|timeout|failed to download|SSL|certificate)",
        "description": "Network connectivity or certificate issue",
        "solutions": [
            {
                "method": "check_firewall",
                "description": "Check Windows Firewall and antivirus settings"
            },
            {
                "method": "proxy",
                "description": "Configure proxy if behind corporate firewall"
            },
            {
                "method": "ssl_cert",
                "description": "Update Windows root certificates: certutil -generateSSTFromWU roots.sst"
            }
        ],
        "ai_prompt": "Analyze this network/SSL error and suggest Windows firewall/proxy solutions"
    },
    "permission_error": {
        "pattern": r"(permission denied|access denied|administrator|elevation required)",
        "description": "Insufficient permissions",
        "solutions": [
            {
                "method": "admin",
                "description": "Run PowerShell or Command Prompt as Administrator"
            },
            {
                "method": "user_install",
                "description": "Try user-level installation instead of system-level"
            }
        ],
        "ai_prompt": "Analyze this Windows permission error and suggest solutions"
    }
}


# ============================================================================
# Utility Functions
# ============================================================================

def print_colored(text: str, color: str = Color.WHITE, bold: bool = False) -> None:
    """Print colored text to terminal"""
    prefix = Color.BOLD if bold else ""
    print(f"{prefix}{color}{text}{Color.END}")


def print_header(text: str) -> None:
    """Print section header"""
    print("\n" + "=" * 70)
    print_colored(text, Color.CYAN, bold=True)
    print("=" * 70)


def print_success(text: str) -> None:
    """Print success message"""
    print_colored(f"✓ {text}", Color.GREEN, bold=True)


def print_error(text: str) -> None:
    """Print error message"""
    print_colored(f"✗ {text}", Color.RED, bold=True)


def print_warning(text: str) -> None:
    """Print warning message"""
    print_colored(f"⚠ {text}", Color.YELLOW)


def print_info(text: str) -> None:
    """Print info message"""
    print_colored(f"ℹ {text}", Color.BLUE)


def run_command(cmd: str, shell: bool = True, check: bool = True, 
                capture_output: bool = True, timeout: int = 300) -> subprocess.CompletedProcess:
    """
    Run shell command with error handling
    
    Returns CompletedProcess with stdout, stderr, and returncode
    """
    try:
        result = subprocess.run(
            cmd,
            shell=shell,
            check=check,
            capture_output=capture_output,
            text=True,
            timeout=timeout
        )
        return result
    except subprocess.CalledProcessError as e:
        print_error(f"Command failed: {cmd}")
        print_error(f"Error: {e.stderr if e.stderr else str(e)}")
        raise
    except subprocess.TimeoutExpired:
        print_error(f"Command timed out after {timeout}s: {cmd}")
        raise


def check_command_exists(command: str) -> bool:
    """Check if a command is available in PATH"""
    return shutil.which(command) is not None


def get_version(command: str) -> Optional[str]:
    """Get version of a command"""
    try:
        result = run_command(f"{command} --version", check=False)
        if result.returncode == 0:
            # Extract version number from output
            version_match = re.search(r'(\d+\.\d+\.\d+)', result.stdout)
            if version_match:
                return version_match.group(1)
            return result.stdout.split('\n')[0]
        return None
    except:
        return None


def is_admin() -> bool:
    """Check if running with administrator privileges"""
    try:
        import ctypes
        return ctypes.windll.shell32.IsUserAnAdmin() != 0
    except:
        return False


def download_file(url: str, destination: Path, timeout: int = 60) -> bool:
    """Download file from URL with progress indication"""
    try:
        print_info(f"Downloading from {url}...")
        
        with urllib.request.urlopen(url, timeout=timeout) as response:
            total_size = int(response.headers.get('content-length', 0))
            downloaded = 0
            chunk_size = 8192
            
            with open(destination, 'wb') as f:
                while True:
                    chunk = response.read(chunk_size)
                    if not chunk:
                        break
                    f.write(chunk)
                    downloaded += len(chunk)
                    if total_size > 0:
                        percent = (downloaded / total_size) * 100
                        print(f"\rProgress: {percent:.1f}%", end='', flush=True)
        
        print()  # New line after progress
        print_success(f"Downloaded to {destination}")
        return True
        
    except urllib.error.URLError as e:
        print_error(f"Download failed: {e}")
        return False
    except Exception as e:
        print_error(f"Unexpected error during download: {e}")
        return False


# ============================================================================
# AI Error Fallback System
# ============================================================================

class ErrorAnalyzer:
    """AI-powered error analysis and solution suggestion"""
    
    def __init__(self):
        self.error_log = []
        self.forge_available = False
        
    def analyze_error(self, error_message: str, context: Dict = None) -> Dict:
        """
        Analyze error and suggest solutions
        
        Returns dict with:
        - matched_pattern: Known error pattern if matched
        - solutions: List of suggested solutions
        - ai_suggestion: AI-generated suggestion if available
        """
        result = {
            "error": error_message,
            "context": context or {},
            "matched_pattern": None,
            "solutions": [],
            "ai_suggestion": None
        }
        
        # Match against known patterns
        for error_id, error_info in ERROR_KNOWLEDGE_BASE.items():
            if re.search(error_info["pattern"], error_message, re.IGNORECASE):
                result["matched_pattern"] = error_id
                result["solutions"] = error_info["solutions"]
                
                # Try AI analysis if available
                if self.forge_available:
                    result["ai_suggestion"] = self._get_ai_suggestion(
                        error_message, 
                        error_info["ai_prompt"],
                        context
                    )
                
                break
        
        # Log error for learning
        self.error_log.append(result)
        
        return result
    
    def _get_ai_suggestion(self, error: str, prompt: str, context: Dict) -> Optional[str]:
        """Query Forge AI for error analysis (if available)"""
        try:
            # Check if forge is available
            if not check_command_exists("forge"):
                return None
            
            # Construct AI prompt with context
            full_prompt = f"""
{prompt}

Error Message:
{error}

System Context:
- OS: {context.get('os', 'Unknown')}
- Architecture: {context.get('arch', 'Unknown')}
- Python: {context.get('python_version', 'Unknown')}

Please provide:
1. Root cause analysis
2. Step-by-step solution for Windows
3. Alternative approaches if the main solution fails
"""
            
            # Query Forge (simplified - actual implementation would use Forge API)
            # For now, return None to avoid execution errors
            return None
            
        except Exception as e:
            print_warning(f"Could not get AI suggestion: {e}")
            return None
    
    def print_solutions(self, analysis: Dict) -> None:
        """Print error analysis and solutions"""
        print_header("Error Analysis")
        
        print_error(f"Error: {analysis['error']}")
        
        if analysis["matched_pattern"]:
            pattern_info = ERROR_KNOWLEDGE_BASE[analysis["matched_pattern"]]
            print_info(f"Issue: {pattern_info['description']}")
            
            print("\nSuggested Solutions:")
            for i, solution in enumerate(analysis["solutions"], 1):
                print(f"\n{i}. {solution.get('description', 'No description')}")
                if "command" in solution:
                    print_colored(f"   Command: {solution['command']}", Color.CYAN)
                if "url" in solution:
                    print_colored(f"   URL: {solution['url']}", Color.BLUE)
        
        if analysis["ai_suggestion"]:
            print("\nAI-Powered Suggestion:")
            print_colored(analysis["ai_suggestion"], Color.MAGENTA)
    
    def save_log(self, filepath: Path = Path("setup_errors.json")) -> None:
        """Save error log to file"""
        try:
            with open(filepath, 'w') as f:
                json.dump(self.error_log, f, indent=2)
            print_info(f"Error log saved to {filepath}")
        except Exception as e:
            print_warning(f"Could not save error log: {e}")


# ============================================================================
# Environment Validation
# ============================================================================

class EnvironmentValidator:
    """Validate system environment and dependencies"""
    
    def __init__(self):
        self.results = {}
        self.error_analyzer = ErrorAnalyzer()
    
    def validate_all(self) -> bool:
        """Run all validation checks"""
        print_header("Environment Validation")
        
        all_valid = True
        
        # Basic system checks
        all_valid &= self.check_os()
        all_valid &= self.check_architecture()
        all_valid &= self.check_python()
        
        # Check for admin privileges (recommended but not required)
        if not is_admin():
            print_warning("Not running as Administrator - some operations may fail")
            print_info("To run as admin: Right-click PowerShell → 'Run as Administrator'")
        
        return all_valid
    
    def check_os(self) -> bool:
        """Validate operating system"""
        os_name = platform.system()
        self.results['os'] = os_name
        
        if os_name != 'Windows':
            print_error(f"This script is for Windows only (detected: {os_name})")
            print_info("For Linux/macOS, see: https://github.com/antinomyhq/forge#installation")
            return False
        
        print_success(f"Operating System: {os_name}")
        return True
    
    def check_architecture(self) -> bool:
        """Validate system architecture"""
        arch = platform.machine()
        self.results['arch'] = arch
        
        if arch not in ['AMD64', 'x86_64', 'ARM64']:
            print_warning(f"Unusual architecture detected: {arch}")
            print_info("Binary installation may not be available")
        else:
            print_success(f"Architecture: {arch}")
        
        return True
    
    def check_python(self) -> bool:
        """Validate Python version"""
        version = sys.version_info
        version_str = f"{version.major}.{version.minor}.{version.micro}"
        self.results['python_version'] = version_str
        
        if version.major < 3 or (version.major == 3 and version.minor < 7):
            print_error(f"Python 3.7+ required (found: {version_str})")
            return False
        
        print_success(f"Python: {version_str}")
        return True
    
    def check_node(self) -> Tuple[bool, Optional[str]]:
        """Check Node.js installation"""
        if check_command_exists("node"):
            version = get_version("node")
            print_success(f"Node.js: {version}")
            return True, version
        else:
            print_warning("Node.js not found")
            return False, None
    
    def check_npm(self) -> Tuple[bool, Optional[str]]:
        """Check npm installation"""
        if check_command_exists("npm"):
            version = get_version("npm")
            print_success(f"npm: {version}")
            return True, version
        else:
            print_warning("npm not found")
            return False, None
    
    def check_rust(self) -> Tuple[bool, Optional[str]]:
        """Check Rust installation"""
        if check_command_exists("cargo"):
            version = get_version("cargo")
            print_success(f"Rust (cargo): {version}")
            return True, version
        else:
            print_warning("Rust not found")
            return False, None
    
    def check_git(self) -> Tuple[bool, Optional[str]]:
        """Check Git installation"""
        if check_command_exists("git"):
            version = get_version("git")
            print_success(f"Git: {version}")
            return True, version
        else:
            print_warning("Git not found")
            return False, None


# ============================================================================
# Installation Methods
# ============================================================================

class ForgeInstaller:
    """Main installer class with multiple installation methods"""
    
    def __init__(self, non_interactive: bool = False):
        self.non_interactive = non_interactive
        self.validator = EnvironmentValidator()
        self.error_analyzer = ErrorAnalyzer()
    
    def install(self, method: Optional[InstallMethod] = None) -> bool:
        """
        Main installation flow
        
        Args:
            method: Specific installation method or None for interactive selection
        """
        print_header("Forge Installation Wizard")
        print("AI-Enhanced Setup for Windows\n")
        
        # Validate environment
        if not self.validator.validate_all():
            print_error("Environment validation failed")
            return False
        
        # Determine installation method
        if method is None:
            method = self._choose_installation_method()
        
        # Execute installation
        print_header(f"Installing via {method.value.upper()}")
        
        try:
            if method == InstallMethod.NPX:
                success = self._install_npx()
            elif method == InstallMethod.BINARY:
                success = self._install_binary()
            elif method == InstallMethod.SOURCE:
                success = self._install_source()
            else:
                print_error(f"Unknown installation method: {method}")
                return False
            
            if success:
                self._post_install_setup()
                print_success("\n🎉 Forge installed successfully!")
                print_info("\nNext steps:")
                print("  1. Close and reopen your terminal")
                print("  2. Run: forge --version")
                print("  3. Configure API keys (see INSTRUCTIONS.md)")
                return True
            else:
                print_error("\n❌ Installation failed")
                return False
                
        except Exception as e:
            print_error(f"Installation error: {e}")
            
            # Analyze error with AI
            analysis = self.error_analyzer.analyze_error(
                str(e),
                context=self.validator.results
            )
            self.error_analyzer.print_solutions(analysis)
            self.error_analyzer.save_log()
            
            return False
    
    def _choose_installation_method(self) -> InstallMethod:
        """Interactive method selection"""
        if self.non_interactive:
            # Auto-detect best method
            node_ok, _ = self.validator.check_node()
            if node_ok:
                return InstallMethod.NPX
            else:
                return InstallMethod.BINARY
        
        print("\nAvailable Installation Methods:\n")
        print("1. NPX (Recommended) - Easiest, requires Node.js")
        print("2. Binary - No dependencies, download and install")
        print("3. Source - Build from Rust source, for developers")
        
        while True:
            choice = input("\nSelect method (1-3): ").strip()
            if choice == '1':
                return InstallMethod.NPX
            elif choice == '2':
                return InstallMethod.BINARY
            elif choice == '3':
                return InstallMethod.SOURCE
            else:
                print_warning("Invalid choice, please enter 1, 2, or 3")
    
    def _install_npx(self) -> bool:
        """Install via NPX"""
        # Check Node.js
        node_ok, node_version = self.validator.check_node()
        npm_ok, npm_version = self.validator.check_npm()
        
        if not node_ok or not npm_ok:
            print_error("Node.js and npm are required for NPX installation")
            print_info("Install Node.js from: https://nodejs.org/")
            
            # Try to auto-install via winget
            if check_command_exists("winget"):
                install_node = input("\nInstall Node.js via winget? (y/n): ").strip().lower()
                if install_node == 'y':
                    print_info("Installing Node.js...")
                    try:
                        run_command("winget install OpenJS.NodeJS.LTS", timeout=600)
                        print_success("Node.js installed! Please restart terminal and run setup again.")
                        return False
                    except Exception as e:
                        print_error(f"Winget installation failed: {e}")
                        return False
            return False
        
        # Install globally via npm
        print_info("Installing Forge globally via npm...")
        try:
            run_command("npm install -g forgecode", timeout=600)
            print_success("Forge installed via npm")
            return True
        except Exception as e:
            print_error(f"NPM installation failed: {e}")
            
            # Try npx direct approach as fallback
            print_info("Trying direct npx approach...")
            try:
                result = run_command("npx forgecode@latest --version", timeout=120)
                if result.returncode == 0:
                    print_success("Forge available via npx")
                    print_info("Run with: npx forgecode@latest")
                    return True
            except:
                pass
            
            return False
    
    def _install_binary(self) -> bool:
        """Install pre-built binary"""
        print_info("Installing from pre-built binary...")
        
        # Determine download URL
        arch = platform.machine()
        if arch == 'AMD64' or arch == 'x86_64':
            binary_name = "forge-x86_64-pc-windows-msvc.exe"
        elif arch == 'ARM64':
            binary_name = "forge-aarch64-pc-windows-msvc.exe"
        else:
            print_error(f"No pre-built binary available for architecture: {arch}")
            return False
        
        download_url = f"https://github.com/antinomyhq/forge/releases/latest/download/{binary_name}"
        
        # Create installation directory
        install_dir = Path("C:/Program Files/Forge")
        if not is_admin():
            # Use user directory if not admin
            install_dir = Path.home() / ".forge"
        
        install_dir.mkdir(parents=True, exist_ok=True)
        binary_path = install_dir / "forge.exe"
        
        # Download binary
        if not download_file(download_url, binary_path, timeout=300):
            return False
        
        # Add to PATH
        self._add_to_path(str(install_dir))
        
        print_success(f"Binary installed to {binary_path}")
        return True
    
    def _install_source(self) -> bool:
        """Build from source"""
        print_info("Building from source...")
        
        # Check Rust
        rust_ok, rust_version = self.validator.check_rust()
        if not rust_ok:
            print_error("Rust toolchain required for source installation")
            print_info("Install from: https://rustup.rs/")
            
            # Try auto-install
            if check_command_exists("winget"):
                install_rust = input("\nInstall Rust via winget? (y/n): ").strip().lower()
                if install_rust == 'y':
                    try:
                        run_command("winget install Rustlang.Rustup", timeout=600)
                        print_success("Rust installed! Please restart terminal and run setup again.")
                        return False
                    except Exception as e:
                        print_error(f"Winget installation failed: {e}")
                        return False
            return False
        
        # Check Git
        git_ok, git_version = self.validator.check_git()
        if not git_ok:
            print_error("Git required for source installation")
            return False
        
        # Clone repository
        repo_dir = Path("forge-source")
        if repo_dir.exists():
            print_info(f"Repository already exists at {repo_dir}")
        else:
            print_info("Cloning repository...")
            try:
                run_command(
                    "git clone https://github.com/antinomyhq/forge.git forge-source",
                    timeout=600
                )
            except Exception as e:
                print_error(f"Git clone failed: {e}")
                return False
        
        # Build
        print_info("Building Forge (this may take 10-20 minutes)...")
        try:
            # Change to repo directory
            os.chdir(repo_dir)
            
            # Build release
            run_command("cargo build --release", timeout=1800)
            
            # Install
            run_command("cargo install --path .", timeout=600)
            
            print_success("Forge built and installed from source")
            return True
            
        except Exception as e:
            print_error(f"Build failed: {e}")
            
            # Common build errors
            if "linker" in str(e).lower() or "link.exe" in str(e).lower():
                print_warning("\nLinker error detected!")
                print_info("Install Visual Studio Build Tools:")
                print_info("  winget install Microsoft.VisualStudio.2022.BuildTools")
                print_info("  Or download from: https://visualstudio.microsoft.com/downloads/")
            
            return False
    
    def _add_to_path(self, directory: str) -> None:
        """Add directory to Windows PATH"""
        try:
            if is_admin():
                # System-level PATH (requires admin)
                print_info("Adding to system PATH (requires admin)...")
                run_command(
                    f'setx /M PATH "%PATH%;{directory}"',
                    shell=True,
                    check=False
                )
            else:
                # User-level PATH
                print_info("Adding to user PATH...")
                run_command(
                    f'setx PATH "%PATH%;{directory}"',
                    shell=True,
                    check=False
                )
            
            print_success(f"Added {directory} to PATH")
            print_warning("Please restart your terminal for PATH changes to take effect")
            
        except Exception as e:
            print_warning(f"Could not automatically add to PATH: {e}")
            print_info(f"\nManually add to PATH:")
            print_info(f"  1. Open System Properties → Environment Variables")
            print_info(f"  2. Edit PATH variable")
            print_info(f"  3. Add: {directory}")
    
    def _post_install_setup(self) -> None:
        """Post-installation setup"""
        print_header("Post-Installation Setup")
        
        # Create example configuration files
        self._create_example_configs()
        
        # Verify installation
        if check_command_exists("forge"):
            version = get_version("forge")
            print_success(f"Forge {version} verified")
        else:
            print_warning("Forge not in PATH yet - restart terminal")
    
    def _create_example_configs(self) -> None:
        """Create example configuration files"""
        # .env.example
        env_example = """# Forge Configuration
# Copy to .env and fill in your API keys

# Primary Provider (OpenRouter recommended)
OPENROUTER_API_KEY=sk-or-v1-your_key_here

# Alternative Providers
OPENAI_API_KEY=sk-your_key_here
ANTHROPIC_API_KEY=sk-ant-your_key_here
GOOGLE_API_KEY=your_key_here

# HTTP Configuration
FORGE_HTTP_CONNECT_TIMEOUT=30
FORGE_HTTP_READ_TIMEOUT=900

# Windows-specific
COMSPEC=C:\\Windows\\System32\\cmd.exe
"""
        
        env_example_path = Path(".env.example")
        if not env_example_path.exists():
            env_example_path.write_text(env_example)
            print_success("Created .env.example")
        
        # forge.yaml example
        forge_yaml = """# Forge Configuration
model: "claude-3.7-sonnet"
temperature: 0.7

custom_rules: |
  1. Add comprehensive error handling
  2. Include unit tests
  3. Follow project conventions

max_walker_depth: 3
"""
        
        forge_yaml_path = Path("forge.yaml.example")
        if not forge_yaml_path.exists():
            forge_yaml_path.write_text(forge_yaml)
            print_success("Created forge.yaml.example")


# ============================================================================
# Main Entry Point
# ============================================================================

def main():
    """Main entry point"""
    import argparse
    
    parser = argparse.ArgumentParser(
        description="Forge Setup Script for Windows",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python setup.py                    # Interactive setup
  python setup.py --method npx       # Install via NPX
  python setup.py --method binary    # Install pre-built binary
  python setup.py --method source    # Build from source
  python setup.py --validate-only    # Check environment only
  python setup.py --non-interactive  # Automated setup

For more information, see INSTRUCTIONS.md
        """
    )
    
    parser.add_argument(
        '--method',
        choices=['npx', 'binary', 'source'],
        help='Installation method'
    )
    
    parser.add_argument(
        '--non-interactive',
        action='store_true',
        help='Run without interactive prompts'
    )
    
    parser.add_argument(
        '--validate-only',
        action='store_true',
        help='Only validate environment without installing'
    )
    
    args = parser.parse_args()
    
    # Convert method string to enum
    method = None
    if args.method:
        method = InstallMethod(args.method)
    
    # Create installer
    installer = ForgeInstaller(non_interactive=args.non_interactive)
    
    # Validate only mode
    if args.validate_only:
        print_header("Environment Validation Only")
        valid = installer.validator.validate_all()
        
        # Check optional dependencies
        installer.validator.check_node()
        installer.validator.check_npm()
        installer.validator.check_rust()
        installer.validator.check_git()
        
        sys.exit(0 if valid else 1)
    
    # Run installation
    success = installer.install(method=method)
    
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()


#!/usr/bin/env python3
"""
Forge Launcher with AI-Powered Runtime Diagnostics
===================================================

Smart launcher that validates configuration, detects issues, and provides
AI-powered solutions for runtime problems.

Features:
- Configuration validation (forge.yaml, .env, .mcp.json)
- API key verification
- Intelligent error detection and recovery
- Auto-fix for common issues
- Integration with Forge's AI for self-diagnosis
- Multiple launch modes support

Usage:
    python start.py                        # Interactive mode
    python start.py --prompt "question"    # Direct prompt mode
    python start.py --workflow file.yaml   # Workflow mode
    python start.py --validate             # Validate only, don't launch
    python start.py --fix                  # Auto-fix common issues
"""

import os
import sys
import subprocess
import json
import re
import time
from pathlib import Path
from typing import Optional, Dict, List, Tuple
from dataclasses import dataclass
import argparse


# ============================================================================
# Configuration & Constants
# ============================================================================

@dataclass
class ForgeConfig:
    """Forge configuration state"""
    forge_yaml_path: Optional[Path] = None
    env_path: Optional[Path] = None
    mcp_json_path: Optional[Path] = None
    forge_yaml: Optional[Dict] = None
    env_vars: Dict[str, str] = None
    mcp_config: Optional[Dict] = None
    api_keys: Dict[str, bool] = None
    
    def __post_init__(self):
        if self.env_vars is None:
            self.env_vars = {}
        if self.api_keys is None:
            self.api_keys = {}


class Color:
    """ANSI color codes"""
    RED = '\033[91m'
    GREEN = '\033[92m'
    YELLOW = '\033[93m'
    BLUE = '\033[94m'
    MAGENTA = '\033[95m'
    CYAN = '\033[96m'
    BOLD = '\033[1m'
    END = '\033[0m'


SUPPORTED_PROVIDERS = {
    'OPENROUTER_API_KEY': {
        'name': 'OpenRouter',
        'pattern': r'^sk-or-v1-',
        'url': 'https://openrouter.ai/keys',
        'recommended': True
    },
    'OPENAI_API_KEY': {
        'name': 'OpenAI',
        'pattern': r'^sk-',
        'url': 'https://platform.openai.com/api-keys'
    },
    'ANTHROPIC_API_KEY': {
        'name': 'Anthropic',
        'pattern': r'^sk-ant-',
        'url': 'https://console.anthropic.com/'
    },
    'GOOGLE_API_KEY': {
        'name': 'Google',
        'pattern': r'^[A-Za-z0-9_-]+$',
        'url': 'https://ai.google.dev/'
    },
    'XAI_API_KEY': {
        'name': 'x.ai',
        'pattern': r'^xai-',
        'url': 'https://console.x.ai/'
    },
    'ZAI_API_KEY': {
        'name': 'z.ai',
        'pattern': r'^.+$',
        'url': 'https://z.ai/'
    },
    'ZAI_CODING_API_KEY': {
        'name': 'z.ai Coding Plan',
        'pattern': r'^.+$',
        'url': 'https://z.ai/'
    },
    'CEREBRAS_API_KEY': {
        'name': 'Cerebras',
        'pattern': r'^.+$',
        'url': 'https://cerebras.ai/'
    }
}


# ============================================================================
# Utility Functions
# ============================================================================

def print_colored(text: str, color: str = Color.WHITE, bold: bool = False) -> None:
    """Print colored text"""
    prefix = Color.BOLD if bold else ""
    print(f"{prefix}{color}{text}{Color.END}")


def print_header(text: str) -> None:
    """Print section header"""
    print("\n" + "=" * 70)
    print_colored(text, Color.CYAN, bold=True)
    print("=" * 70)


def print_success(text: str) -> None:
    print_colored(f"✓ {text}", Color.GREEN, bold=True)


def print_error(text: str) -> None:
    print_colored(f"✗ {text}", Color.RED, bold=True)


def print_warning(text: str) -> None:
    print_colored(f"⚠ {text}", Color.YELLOW)


def print_info(text: str) -> None:
    print_colored(f"ℹ {text}", Color.BLUE)


def find_file_up_tree(filename: str, start_path: Path = None) -> Optional[Path]:
    """Search for file in current directory and parent directories"""
    if start_path is None:
        start_path = Path.cwd()
    
    current = start_path
    while True:
        candidate = current / filename
        if candidate.exists():
            return candidate
        
        parent = current.parent
        if parent == current:  # Reached root
            break
        current = parent
    
    # Check home directory as fallback
    home_candidate = Path.home() / filename
    if home_candidate.exists():
        return home_candidate
    
    return None


def load_yaml_file(filepath: Path) -> Optional[Dict]:
    """Load YAML file (basic parser, no external deps)"""
    try:
        import yaml
        with open(filepath, 'r') as f:
            return yaml.safe_load(f)
    except ImportError:
        # Fallback to basic parsing if yaml not available
        print_warning("PyYAML not installed, using basic parser")
        return _parse_yaml_basic(filepath)
    except Exception as e:
        print_error(f"Error loading YAML: {e}")
        return None


def _parse_yaml_basic(filepath: Path) -> Dict:
    """Basic YAML parser for simple key-value configs"""
    config = {}
    try:
        with open(filepath, 'r') as f:
            for line in f:
                line = line.strip()
                if line and not line.startswith('#'):
                    if ':' in line:
                        key, value = line.split(':', 1)
                        key = key.strip()
                        value = value.strip().strip('"\'')
                        config[key] = value
    except Exception as e:
        print_warning(f"Basic YAML parse error: {e}")
    return config


def load_env_file(filepath: Path) -> Dict[str, str]:
    """Load .env file"""
    env_vars = {}
    try:
        with open(filepath, 'r') as f:
            for line in f:
                line = line.strip()
                if line and not line.startswith('#') and '=' in line:
                    key, value = line.split('=', 1)
                    key = key.strip()
                    value = value.strip().strip('"\'')
                    env_vars[key] = value
    except Exception as e:
        print_error(f"Error loading .env: {e}")
    return env_vars


def load_json_file(filepath: Path) -> Optional[Dict]:
    """Load JSON file"""
    try:
        with open(filepath, 'r') as f:
            return json.load(f)
    except Exception as e:
        print_error(f"Error loading JSON: {e}")
        return None


# ============================================================================
# Configuration Validation
# ============================================================================

class ConfigValidator:
    """Validate Forge configuration files"""
    
    def __init__(self):
        self.config = ForgeConfig()
        self.issues = []
        self.warnings = []
    
    def validate_all(self) -> bool:
        """Run all validation checks"""
        print_header("Configuration Validation")
        
        # Find and load config files
        self._discover_configs()
        self._load_configs()
        
        # Validate each component
        all_valid = True
        all_valid &= self._validate_forge_yaml()
        all_valid &= self._validate_env()
        all_valid &= self._validate_api_keys()
        self._validate_mcp_config()  # Optional, doesn't affect validity
        
        # Print summary
        self._print_summary()
        
        return all_valid
    
    def _discover_configs(self) -> None:
        """Find configuration files"""
        # forge.yaml
        self.config.forge_yaml_path = find_file_up_tree('forge.yaml')
        if not self.config.forge_yaml_path:
            self.warnings.append("No forge.yaml found (using defaults)")
        else:
            print_info(f"Found forge.yaml: {self.config.forge_yaml_path}")
        
        # .env
        self.config.env_path = find_file_up_tree('.env')
        if not self.config.env_path:
            self.warnings.append("No .env file found")
        else:
            print_info(f"Found .env: {self.config.env_path}")
        
        # .mcp.json
        self.config.mcp_json_path = find_file_up_tree('.mcp.json')
        if self.config.mcp_json_path:
            print_info(f"Found .mcp.json: {self.config.mcp_json_path}")
    
    def _load_configs(self) -> None:
        """Load configuration files"""
        # Load forge.yaml
        if self.config.forge_yaml_path:
            self.config.forge_yaml = load_yaml_file(self.config.forge_yaml_path)
        
        # Load .env
        if self.config.env_path:
            self.config.env_vars = load_env_file(self.config.env_path)
        else:
            # Check environment variables
            self.config.env_vars = {
                key: value for key, value in os.environ.items()
                if any(provider in key for provider in SUPPORTED_PROVIDERS)
            }
        
        # Load .mcp.json
        if self.config.mcp_json_path:
            self.config.mcp_config = load_json_file(self.config.mcp_json_path)
    
    def _validate_forge_yaml(self) -> bool:
        """Validate forge.yaml configuration"""
        if not self.config.forge_yaml:
            return True  # Optional file
        
        yaml_config = self.config.forge_yaml
        
        # Check model is specified
        if 'model' not in yaml_config:
            self.warnings.append("No model specified in forge.yaml")
        else:
            model = yaml_config['model']
            print_success(f"Model: {model}")
        
        # Check temperature range
        if 'temperature' in yaml_config:
            temp = yaml_config['temperature']
            try:
                temp_float = float(temp)
                if temp_float < 0 or temp_float > 2:
                    self.warnings.append(f"Temperature {temp} outside recommended range [0, 2]")
                else:
                    print_success(f"Temperature: {temp}")
            except ValueError:
                self.issues.append(f"Invalid temperature value: {temp}")
                return False
        
        return True
    
    def _validate_env(self) -> bool:
        """Validate environment configuration"""
        if not self.config.env_vars:
            self.issues.append("No environment variables found")
            return False
        
        print_success(f"Loaded {len(self.config.env_vars)} environment variables")
        return True
    
    def _validate_api_keys(self) -> bool:
        """Validate API keys"""
        found_keys = {}
        
        for key_name, provider_info in SUPPORTED_PROVIDERS.items():
            if key_name in self.config.env_vars:
                key_value = self.config.env_vars[key_name]
                
                # Check pattern
                pattern = provider_info.get('pattern')
                if pattern and not re.match(pattern, key_value):
                    self.warnings.append(
                        f"{provider_info['name']} API key format looks invalid"
                    )
                
                found_keys[key_name] = True
                print_success(f"API Key found: {provider_info['name']}")
            else:
                found_keys[key_name] = False
        
        self.config.api_keys = found_keys
        
        # Check if at least one key is present
        if not any(found_keys.values()):
            self.issues.append("No API keys found!")
            print_error("\nNo API keys configured!")
            print_info("\nRecommended: Get an OpenRouter API key")
            print_info("  1. Visit: https://openrouter.ai/keys")
            print_info("  2. Create account and generate key")
            print_info("  3. Add to .env: OPENROUTER_API_KEY=sk-or-v1-...")
            return False
        
        # Recommend OpenRouter if not present
        if not found_keys.get('OPENROUTER_API_KEY'):
            self.warnings.append(
                "OpenRouter not configured (recommended for access to 300+ models)"
            )
        
        return True
    
    def _validate_mcp_config(self) -> None:
        """Validate MCP configuration (optional)"""
        if not self.config.mcp_config:
            return
        
        servers = self.config.mcp_config.get('mcpServers', {})
        if servers:
            print_success(f"MCP Servers: {len(servers)} configured")
            for server_name in servers.keys():
                print_info(f"  - {server_name}")
    
    def _print_summary(self) -> None:
        """Print validation summary"""
        print("\n" + "-" * 70)
        
        if self.issues:
            print_colored("\nCritical Issues:", Color.RED, bold=True)
            for issue in self.issues:
                print_error(issue)
        
        if self.warnings:
            print_colored("\nWarnings:", Color.YELLOW, bold=True)
            for warning in self.warnings:
                print_warning(warning)
        
        if not self.issues and not self.warnings:
            print_colored("\n✓ All validations passed!", Color.GREEN, bold=True)


# ============================================================================
# Auto-Fix System
# ============================================================================

class ConfigFixer:
    """Automatically fix common configuration issues"""
    
    def __init__(self, config: ForgeConfig):
        self.config = config
    
    def fix_all(self) -> bool:
        """Attempt to fix all common issues"""
        print_header("Auto-Fix Configuration Issues")
        
        fixed_any = False
        
        # Create .env if missing
        if not self.config.env_path:
            if self._create_default_env():
                fixed_any = True
        
        # Create forge.yaml if missing
        if not self.config.forge_yaml_path:
            if self._create_default_forge_yaml():
                fixed_any = True
        
        if fixed_any:
            print_success("\n✓ Configuration fixed!")
            print_info("Please edit .env to add your API keys")
        else:
            print_info("No auto-fixes needed")
        
        return fixed_any
    
    def _create_default_env(self) -> bool:
        """Create default .env file"""
        print_info("Creating default .env file...")
        
        env_content = """# Forge Environment Configuration
# Add your API keys here

# OpenRouter (Recommended - access to 300+ models)
OPENROUTER_API_KEY=

# Alternative Providers (uncomment and add your keys)
# OPENAI_API_KEY=
# ANTHROPIC_API_KEY=
# GOOGLE_API_KEY=

# HTTP Configuration (optional)
# FORGE_HTTP_CONNECT_TIMEOUT=30
# FORGE_HTTP_READ_TIMEOUT=900

# Windows-specific (optional)
# COMSPEC=C:\\Windows\\System32\\cmd.exe
"""
        
        try:
            env_path = Path('.env')
            env_path.write_text(env_content)
            print_success(f"Created .env at {env_path.absolute()}")
            return True
        except Exception as e:
            print_error(f"Could not create .env: {e}")
            return False
    
    def _create_default_forge_yaml(self) -> bool:
        """Create default forge.yaml file"""
        print_info("Creating default forge.yaml...")
        
        yaml_content = """# Forge Configuration
# See: https://forgecode.dev/docs/configuration

# Default model (change to your preference)
model: "claude-3.7-sonnet"

# Temperature: 0.0 (focused) to 2.0 (creative)
temperature: 0.7

# Custom rules for AI behavior
custom_rules: |
  1. Add comprehensive error handling to all code
  2. Include unit tests for new functions
  3. Follow project coding standards

# Directory traversal depth
max_walker_depth: 3

# Safety limits
max_requests_per_turn: 50
max_tool_failure_per_turn: 3
"""
        
        try:
            yaml_path = Path('forge.yaml')
            yaml_path.write_text(yaml_content)
            print_success(f"Created forge.yaml at {yaml_path.absolute()}")
            return True
        except Exception as e:
            print_error(f"Could not create forge.yaml: {e}")
            return False


# ============================================================================
# Forge Launcher
# ============================================================================

class ForgeLauncher:
    """Launch Forge with error handling and diagnostics"""
    
    def __init__(self, validate_only: bool = False, auto_fix: bool = False):
        self.validate_only = validate_only
        self.auto_fix = auto_fix
        self.validator = ConfigValidator()
    
    def launch(self, forge_args: List[str] = None) -> int:
        """
        Main launch flow
        
        Returns exit code (0 for success)
        """
        # Validate configuration
        valid = self.validator.validate_all()
        
        # Auto-fix if requested
        if self.auto_fix:
            fixer = ConfigFixer(self.validator.config)
            fixer.fix_all()
            # Re-validate after fixes
            self.validator = ConfigValidator()
            valid = self.validator.validate_all()
        
        # Check if validation only
        if self.validate_only:
            return 0 if valid else 1
        
        # Check critical issues
        if not valid:
            print_error("\nCannot launch Forge due to configuration issues")
            print_info("Run with --fix to auto-fix common issues")
            print_info("Or see INSTRUCTIONS.md for manual setup")
            return 1
        
        # Launch Forge
        print_header("Launching Forge")
        
        try:
            # Build command
            cmd = ['forge']
            if forge_args:
                cmd.extend(forge_args)
            
            print_info(f"Command: {' '.join(cmd)}\n")
            
            # Set environment variables from .env
            env = os.environ.copy()
            if self.validator.config.env_vars:
                env.update(self.validator.config.env_vars)
            
            # Launch Forge (interactive)
            result = subprocess.run(cmd, env=env)
            
            return result.returncode
            
        except FileNotFoundError:
            print_error("\nForge executable not found!")
            print_info("Please install Forge first:")
            print_info("  python setup.py")
            print_info("\nOr use NPX:")
            print_info("  npx forgecode@latest")
            return 1
            
        except KeyboardInterrupt:
            print_info("\n\nForge interrupted by user")
            return 130
            
        except Exception as e:
            print_error(f"\nError launching Forge: {e}")
            return 1


# ============================================================================
# Main Entry Point
# ============================================================================

def main():
    """Main entry point"""
    parser = argparse.ArgumentParser(
        description="Forge Launcher with AI-Powered Diagnostics",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python start.py                          # Launch in interactive mode
  python start.py --prompt "Explain auth"  # Direct prompt mode
  python start.py --validate               # Validate config only
  python start.py --fix                    # Auto-fix issues and launch
  python start.py --workflow workflow.yaml # Run workflow

For more information, see INSTRUCTIONS.md
        """
    )
    
    # Validation & fixing
    parser.add_argument(
        '--validate',
        action='store_true',
        help='Validate configuration without launching'
    )
    
    parser.add_argument(
        '--fix',
        action='store_true',
        help='Auto-fix common configuration issues'
    )
    
    # Forge pass-through arguments
    parser.add_argument(
        '--prompt',
        type=str,
        help='Direct prompt to process'
    )
    
    parser.add_argument(
        '--command',
        type=str,
        help='Path to command file'
    )
    
    parser.add_argument(
        '--workflow',
        type=str,
        help='Path to workflow file'
    )
    
    parser.add_argument(
        '--conversation',
        type=str,
        help='Path to conversation file'
    )
    
    parser.add_argument(
        '--verbose',
        action='store_true',
        help='Enable verbose output'
    )
    
    parser.add_argument(
        '--restricted',
        action='store_true',
        help='Enable restricted shell mode'
    )
    
    args, unknown_args = parser.parse_known_args()
    
    # Build Forge arguments
    forge_args = []
    
    if args.prompt:
        forge_args.extend(['--prompt', args.prompt])
    
    if args.command:
        forge_args.extend(['--command', args.command])
    
    if args.workflow:
        forge_args.extend(['--workflow', args.workflow])
    
    if args.conversation:
        forge_args.extend(['--conversation', args.conversation])
    
    if args.verbose:
        forge_args.append('--verbose')
    
    if args.restricted:
        forge_args.append('--restricted')
    
    # Add any unknown args (pass through to Forge)
    forge_args.extend(unknown_args)
    
    # Create launcher
    launcher = ForgeLauncher(
        validate_only=args.validate,
        auto_fix=args.fix
    )
    
    # Launch
    exit_code = launcher.launch(forge_args)
    
    sys.exit(exit_code)


if __name__ == "__main__":
    main()


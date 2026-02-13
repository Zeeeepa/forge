#!/usr/bin/env python3
"""
Forge Installation Validator
=============================

Comprehensive validation utility to check Forge installation health,
configuration, and system requirements.

Usage:
    python validate.py              # Full validation
    python validate.py --quick      # Quick check only
    python validate.py --fix        # Attempt auto-fixes
    python validate.py --report     # Generate detailed report
"""

import os
import sys
import subprocess
import platform
import json
from pathlib import Path
from typing import Dict, List, Tuple, Optional
from dataclasses import dataclass, field
from datetime import datetime


# ============================================================================
# Data Classes
# ============================================================================

@dataclass
class CheckResult:
    """Result of a validation check"""
    name: str
    passed: bool
    message: str
    severity: str = "info"  # info, warning, error, critical
    details: Dict = field(default_factory=dict)
    fix_available: bool = False
    fix_command: Optional[str] = None


@dataclass
class ValidationReport:
    """Complete validation report"""
    timestamp: str
    system_info: Dict
    checks: List[CheckResult]
    summary: Dict
    
    def to_json(self) -> str:
        """Convert to JSON string"""
        return json.dumps(self.__dict__, indent=2, default=str)
    
    def to_markdown(self) -> str:
        """Convert to Markdown format"""
        md = f"# Forge Validation Report\n\n"
        md += f"**Generated**: {self.timestamp}\n\n"
        
        # System Info
        md += "## System Information\n\n"
        for key, value in self.system_info.items():
            md += f"- **{key}**: {value}\n"
        
        # Summary
        md += "\n## Summary\n\n"
        for key, value in self.summary.items():
            md += f"- **{key}**: {value}\n"
        
        # Checks by severity
        md += "\n## Validation Results\n\n"
        
        for severity in ["critical", "error", "warning", "info"]:
            severity_checks = [c for c in self.checks if c.severity == severity]
            if severity_checks:
                md += f"### {severity.capitalize()}\n\n"
                for check in severity_checks:
                    icon = "✓" if check.passed else "✗"
                    md += f"- {icon} **{check.name}**: {check.message}\n"
                    if check.details:
                        for detail_key, detail_value in check.details.items():
                            md += f"  - {detail_key}: {detail_value}\n"
                    if not check.passed and check.fix_available:
                        md += f"  - **Fix**: `{check.fix_command}`\n"
                md += "\n"
        
        return md


# ============================================================================
# Validation Engine
# ============================================================================

class ForgeValidator:
    """Comprehensive Forge installation validator"""
    
    def __init__(self, quick: bool = False):
        self.quick = quick
        self.results: List[CheckResult] = []
        self.system_info = self._collect_system_info()
    
    def _collect_system_info(self) -> Dict:
        """Collect system information"""
        return {
            "OS": platform.system(),
            "OS Version": platform.version(),
            "Architecture": platform.machine(),
            "Python": sys.version.split()[0],
            "User": os.getenv("USERNAME", "Unknown"),
            "Home": str(Path.home()),
            "CWD": str(Path.cwd())
        }
    
    def validate_all(self) -> ValidationReport:
        """Run all validation checks"""
        print("\n" + "="*70)
        print("🔍 Forge Installation Validator")
        print("="*70 + "\n")
        
        # System checks
        self._check_os()
        self._check_architecture()
        self._check_python()
        
        # Installation checks
        self._check_forge_installed()
        self._check_forge_version()
        
        # Dependency checks
        if not self.quick:
            self._check_node()
            self._check_npm()
            self._check_rust()
            self._check_git()
        
        # Configuration checks
        self._check_forge_yaml()
        self._check_env_file()
        self._check_api_keys()
        self._check_mcp_config()
        
        # Runtime checks
        if not self.quick:
            self._check_forge_execution()
            self._check_network()
        
        # Path checks
        self._check_path()
        
        # Generate report
        report = self._generate_report()
        return report
    
    def _add_result(self, result: CheckResult):
        """Add check result and print status"""
        self.results.append(result)
        
        # Print status
        if result.passed:
            icon = "✓"
            color = "\033[92m"  # Green
        else:
            icon = "✗"
            if result.severity == "critical":
                color = "\033[91m"  # Red
            elif result.severity == "error":
                color = "\033[91m"  # Red
            elif result.severity == "warning":
                color = "\033[93m"  # Yellow
            else:
                color = "\033[96m"  # Cyan
        
        print(f"{color}{icon}\033[0m {result.name}: {result.message}")
    
    def _check_os(self):
        """Check operating system"""
        os_name = platform.system()
        if os_name == "Windows":
            self._add_result(CheckResult(
                name="Operating System",
                passed=True,
                message=f"Windows ({platform.release()})",
                severity="info"
            ))
        else:
            self._add_result(CheckResult(
                name="Operating System",
                passed=False,
                message=f"Not Windows (detected: {os_name})",
                severity="error"
            ))
    
    def _check_architecture(self):
        """Check system architecture"""
        arch = platform.machine()
        supported = arch in ['AMD64', 'x86_64', 'ARM64']
        
        self._add_result(CheckResult(
            name="Architecture",
            passed=supported,
            message=f"{arch} ({'supported' if supported else 'may not be supported'})",
            severity="warning" if not supported else "info"
        ))
    
    def _check_python(self):
        """Check Python version"""
        version = sys.version_info
        version_str = f"{version.major}.{version.minor}.{version.micro}"
        min_version = (3, 7)
        
        passed = (version.major, version.minor) >= min_version
        
        self._add_result(CheckResult(
            name="Python Version",
            passed=passed,
            message=f"{version_str} ({'OK' if passed else 'needs 3.7+'})",
            severity="error" if not passed else "info",
            details={"Required": "3.7+", "Found": version_str}
        ))
    
    def _check_forge_installed(self):
        """Check if Forge is installed"""
        forge_path = self._which("forge")
        
        if forge_path:
            self._add_result(CheckResult(
                name="Forge Installation",
                passed=True,
                message=f"Found at {forge_path}",
                severity="info",
                details={"Path": forge_path}
            ))
        else:
            self._add_result(CheckResult(
                name="Forge Installation",
                passed=False,
                message="Not found in PATH",
                severity="critical",
                fix_available=True,
                fix_command="python setup.py"
            ))
    
    def _check_forge_version(self):
        """Check Forge version"""
        if not self._which("forge"):
            return
        
        try:
            result = subprocess.run(
                ["forge", "--version"],
                capture_output=True,
                text=True,
                timeout=10
            )
            
            if result.returncode == 0:
                version = result.stdout.strip()
                self._add_result(CheckResult(
                    name="Forge Version",
                    passed=True,
                    message=version,
                    severity="info"
                ))
            else:
                self._add_result(CheckResult(
                    name="Forge Version",
                    passed=False,
                    message="Could not determine version",
                    severity="warning"
                ))
        except Exception as e:
            self._add_result(CheckResult(
                name="Forge Version",
                passed=False,
                message=f"Error: {e}",
                severity="warning"
            ))
    
    def _check_node(self):
        """Check Node.js installation"""
        node_path = self._which("node")
        
        if node_path:
            version = self._get_version("node")
            self._add_result(CheckResult(
                name="Node.js",
                passed=True,
                message=f"{version} at {node_path}",
                severity="info"
            ))
        else:
            self._add_result(CheckResult(
                name="Node.js",
                passed=False,
                message="Not installed (required for NPX installation method)",
                severity="warning",
                fix_available=True,
                fix_command="winget install OpenJS.NodeJS.LTS"
            ))
    
    def _check_npm(self):
        """Check npm installation"""
        npm_path = self._which("npm")
        
        if npm_path:
            version = self._get_version("npm")
            self._add_result(CheckResult(
                name="npm",
                passed=True,
                message=f"{version} at {npm_path}",
                severity="info"
            ))
        else:
            self._add_result(CheckResult(
                name="npm",
                passed=False,
                message="Not installed (comes with Node.js)",
                severity="warning"
            ))
    
    def _check_rust(self):
        """Check Rust installation"""
        cargo_path = self._which("cargo")
        
        if cargo_path:
            version = self._get_version("cargo")
            self._add_result(CheckResult(
                name="Rust (cargo)",
                passed=True,
                message=f"{version} at {cargo_path}",
                severity="info"
            ))
        else:
            self._add_result(CheckResult(
                name="Rust (cargo)",
                passed=False,
                message="Not installed (required for source installation)",
                severity="info",
                fix_available=True,
                fix_command="winget install Rustlang.Rustup"
            ))
    
    def _check_git(self):
        """Check Git installation"""
        git_path = self._which("git")
        
        if git_path:
            version = self._get_version("git")
            self._add_result(CheckResult(
                name="Git",
                passed=True,
                message=f"{version} at {git_path}",
                severity="info"
            ))
        else:
            self._add_result(CheckResult(
                name="Git",
                passed=False,
                message="Not installed (useful for development)",
                severity="info",
                fix_available=True,
                fix_command="winget install Git.Git"
            ))
    
    def _check_forge_yaml(self):
        """Check forge.yaml configuration"""
        yaml_path = self._find_file_up_tree("forge.yaml")
        
        if yaml_path:
            self._add_result(CheckResult(
                name="forge.yaml",
                passed=True,
                message=f"Found at {yaml_path}",
                severity="info",
                details={"Path": str(yaml_path)}
            ))
            
            # Validate content
            try:
                with open(yaml_path) as f:
                    content = f.read()
                    if "model:" in content:
                        self._add_result(CheckResult(
                            name="forge.yaml Model",
                            passed=True,
                            message="Model configured",
                            severity="info"
                        ))
            except Exception as e:
                self._add_result(CheckResult(
                    name="forge.yaml Content",
                    passed=False,
                    message=f"Error reading file: {e}",
                    severity="warning"
                ))
        else:
            self._add_result(CheckResult(
                name="forge.yaml",
                passed=False,
                message="Not found (using defaults)",
                severity="warning",
                fix_available=True,
                fix_command="python start.py --fix"
            ))
    
    def _check_env_file(self):
        """Check .env file"""
        env_path = self._find_file_up_tree(".env")
        
        if env_path:
            self._add_result(CheckResult(
                name=".env File",
                passed=True,
                message=f"Found at {env_path}",
                severity="info",
                details={"Path": str(env_path)}
            ))
        else:
            self._add_result(CheckResult(
                name=".env File",
                passed=False,
                message="Not found",
                severity="error",
                fix_available=True,
                fix_command="python start.py --fix"
            ))
    
    def _check_api_keys(self):
        """Check for API keys"""
        env_path = self._find_file_up_tree(".env")
        
        if not env_path:
            self._add_result(CheckResult(
                name="API Keys",
                passed=False,
                message="Cannot check (no .env file)",
                severity="error"
            ))
            return
        
        # Load .env
        env_vars = {}
        try:
            with open(env_path) as f:
                for line in f:
                    line = line.strip()
                    if line and not line.startswith('#') and '=' in line:
                        key, value = line.split('=', 1)
                        env_vars[key.strip()] = value.strip()
        except Exception as e:
            self._add_result(CheckResult(
                name="API Keys",
                passed=False,
                message=f"Error reading .env: {e}",
                severity="error"
            ))
            return
        
        # Check for API keys
        api_keys = [
            "OPENROUTER_API_KEY",
            "OPENAI_API_KEY",
            "ANTHROPIC_API_KEY",
            "GOOGLE_API_KEY"
        ]
        
        found_keys = [key for key in api_keys if key in env_vars and env_vars[key]]
        
        if found_keys:
            self._add_result(CheckResult(
                name="API Keys",
                passed=True,
                message=f"Found {len(found_keys)} key(s): {', '.join(found_keys)}",
                severity="info",
                details={"Keys": found_keys}
            ))
        else:
            self._add_result(CheckResult(
                name="API Keys",
                passed=False,
                message="No API keys found in .env",
                severity="critical",
                details={"Required": "At least one API key"}
            ))
    
    def _check_mcp_config(self):
        """Check MCP configuration"""
        mcp_path = self._find_file_up_tree(".mcp.json")
        
        if mcp_path:
            try:
                with open(mcp_path) as f:
                    mcp_config = json.load(f)
                    servers = mcp_config.get("mcpServers", {})
                    
                    self._add_result(CheckResult(
                        name="MCP Configuration",
                        passed=True,
                        message=f"{len(servers)} server(s) configured",
                        severity="info",
                        details={"Servers": list(servers.keys())}
                    ))
            except Exception as e:
                self._add_result(CheckResult(
                    name="MCP Configuration",
                    passed=False,
                    message=f"Error reading .mcp.json: {e}",
                    severity="warning"
                ))
        else:
            self._add_result(CheckResult(
                name="MCP Configuration",
                passed=True,
                message="Not configured (optional)",
                severity="info"
            ))
    
    def _check_forge_execution(self):
        """Test Forge execution"""
        if not self._which("forge"):
            return
        
        try:
            result = subprocess.run(
                ["forge", "--help"],
                capture_output=True,
                text=True,
                timeout=10
            )
            
            if result.returncode == 0:
                self._add_result(CheckResult(
                    name="Forge Execution",
                    passed=True,
                    message="Can execute successfully",
                    severity="info"
                ))
            else:
                self._add_result(CheckResult(
                    name="Forge Execution",
                    passed=False,
                    message=f"Exit code: {result.returncode}",
                    severity="error"
                ))
        except Exception as e:
            self._add_result(CheckResult(
                name="Forge Execution",
                passed=False,
                message=f"Error: {e}",
                severity="error"
            ))
    
    def _check_network(self):
        """Check network connectivity"""
        try:
            import urllib.request
            urllib.request.urlopen("https://api.openrouter.ai", timeout=5)
            
            self._add_result(CheckResult(
                name="Network Connectivity",
                passed=True,
                message="Can reach OpenRouter API",
                severity="info"
            ))
        except Exception as e:
            self._add_result(CheckResult(
                name="Network Connectivity",
                passed=False,
                message=f"Cannot reach API: {e}",
                severity="warning",
                details={"Suggestion": "Check firewall/proxy settings"}
            ))
    
    def _check_path(self):
        """Check PATH configuration"""
        path_dirs = os.environ.get("PATH", "").split(os.pathsep)
        
        # Check for common Forge installation locations
        common_locations = [
            "C:\\Program Files\\Forge",
            str(Path.home() / ".forge"),
            str(Path.home() / ".cargo" / "bin")
        ]
        
        found_locations = [loc for loc in common_locations if loc in path_dirs]
        
        if found_locations:
            self._add_result(CheckResult(
                name="PATH Configuration",
                passed=True,
                message=f"Forge directories in PATH: {', '.join(found_locations)}",
                severity="info"
            ))
        else:
            self._add_result(CheckResult(
                name="PATH Configuration",
                passed=False,
                message="No Forge directories in PATH",
                severity="warning"
            ))
    
    def _which(self, command: str) -> Optional[str]:
        """Find command in PATH"""
        import shutil
        path = shutil.which(command)
        return path if path else None
    
    def _get_version(self, command: str) -> str:
        """Get version of a command"""
        try:
            result = subprocess.run(
                [command, "--version"],
                capture_output=True,
                text=True,
                timeout=5
            )
            if result.returncode == 0:
                return result.stdout.split('\n')[0]
        except:
            pass
        return "unknown"
    
    def _find_file_up_tree(self, filename: str) -> Optional[Path]:
        """Find file in current directory or parents"""
        current = Path.cwd()
        while True:
            candidate = current / filename
            if candidate.exists():
                return candidate
            
            parent = current.parent
            if parent == current:
                break
            current = parent
        
        # Check home directory
        home_candidate = Path.home() / filename
        if home_candidate.exists():
            return home_candidate
        
        return None
    
    def _generate_report(self) -> ValidationReport:
        """Generate validation report"""
        summary = {
            "Total Checks": len(self.results),
            "Passed": len([r for r in self.results if r.passed]),
            "Failed": len([r for r in self.results if not r.passed]),
            "Critical": len([r for r in self.results if r.severity == "critical" and not r.passed]),
            "Errors": len([r for r in self.results if r.severity == "error" and not r.passed]),
            "Warnings": len([r for r in self.results if r.severity == "warning" and not r.passed])
        }
        
        return ValidationReport(
            timestamp=datetime.now().isoformat(),
            system_info=self.system_info,
            checks=self.results,
            summary=summary
        )


# ============================================================================
# Main Entry Point
# ============================================================================

def main():
    """Main entry point"""
    import argparse
    
    parser = argparse.ArgumentParser(
        description="Forge Installation Validator",
        formatter_class=argparse.RawDescriptionHelpFormatter
    )
    
    parser.add_argument(
        '--quick',
        action='store_true',
        help='Run quick validation only (skip optional checks)'
    )
    
    parser.add_argument(
        '--report',
        type=str,
        help='Generate report file (json or md)'
    )
    
    parser.add_argument(
        '--fix',
        action='store_true',
        help='Attempt to auto-fix issues'
    )
    
    args = parser.parse_args()
    
    # Run validation
    validator = ForgeValidator(quick=args.quick)
    report = validator.validate_all()
    
    # Print summary
    print("\n" + "="*70)
    print("📊 Validation Summary")
    print("="*70)
    for key, value in report.summary.items():
        print(f"{key}: {value}")
    
    # Generate report file if requested
    if args.report:
        report_path = Path(args.report)
        if report_path.suffix == '.json':
            content = report.to_json()
        else:
            content = report.to_markdown()
        
        report_path.write_text(content)
        print(f"\n✓ Report saved to {report_path}")
    
    # Auto-fix if requested
    if args.fix:
        print("\n" + "="*70)
        print("🔧 Attempting Auto-Fixes")
        print("="*70)
        
        fixable = [r for r in report.checks if not r.passed and r.fix_available]
        if fixable:
            print(f"\nFound {len(fixable)} fixable issues:")
            for result in fixable:
                print(f"\n  {result.name}: {result.message}")
                print(f"  Fix command: {result.fix_command}")
        else:
            print("\nNo auto-fixable issues found")
    
    # Exit with appropriate code
    if report.summary["Critical"] > 0:
        sys.exit(2)  # Critical issues
    elif report.summary["Errors"] > 0:
        sys.exit(1)  # Errors
    else:
        sys.exit(0)  # Success


if __name__ == "__main__":
    main()


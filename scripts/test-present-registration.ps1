[CmdletBinding()]
param([string]$Fork = '')

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path $PSScriptRoot -Parent
if (-not $Fork) { $Fork = Join-Path $repoRoot 'external\sk8v' }
$sourcePath = Join-Path $Fork 'host\src\board_stencil.cpp'
$source = [IO.File]::ReadAllText($sourcePath)

# Exercise the production bodies, not a second implementation of their logic.
# These two functions have plain balanced blocks and no braced string literals.
function Get-FunctionBody([string]$signature) {
    $start = $source.IndexOf($signature, [StringComparison]::Ordinal)
    if ($start -lt 0) { throw "Production function not found: $signature" }
    $open = $source.IndexOf('{', $start)
    $depth = 1
    for ($i = $open + 1; $i -lt $source.Length; $i++) {
        if ($source[$i] -eq '{') { $depth++ }
        if ($source[$i] -eq '}') { $depth-- }
        if ($depth -eq 0) { return $source.Substring($start, $i - $start + 1) }
    }
    throw "Unbalanced production function: $signature"
}
$startFunction = Get-FunctionBody 'void Start(void (*log)(const char*))'
$stopFunction = Get-FunctionBody 'void Stop()'
$buildRoot = Join-Path $repoRoot ('build\qa-present-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $buildRoot -Force | Out-Null
Copy-Item -LiteralPath $sourcePath -Destination (Join-Path $buildRoot 'board_stencil.source.cpp')

$harness = @'
#include <atomic>
#include <iostream>
#include <stdexcept>
using Callback = void (*)(void*);
static Callback registeredCallback = nullptr;
static unsigned registrations = 0, unregistrations = 0, detourRemovals = 0;
static bool g_registered = false;
static std::atomic<bool> g_active{false}, g_installed{false};
static void (*g_log)(const char*) = nullptr;
void OnPresent(void*) {}
void RemoveDetours() { ++detourRemovals; }
void presentCallbackRegister(Callback fn) {
    if (registeredCallback) throw std::runtime_error("SHV fatal: callback already registered");
    registeredCallback = fn;
    ++registrations;
}
void presentCallbackUnregister(Callback fn) {
    if (!registeredCallback || registeredCallback != fn)
        throw std::runtime_error("SHV fatal: callback not registered");
    registeredCallback = nullptr;
    ++unregistrations;
}
void Check(bool ok, const char* why) { if (!ok) throw std::runtime_error(why); }
void LogA(const char*) {}
void LogB(const char*) {}
'@
$harness += "`n$startFunction`n$stopFunction`n"
$harness += @'
int main() {
    try {
        Stop();
        Check(registrations == 0 && unregistrations == 0, "Stop before Start must not unregister");
        Start(LogA);
        Check(g_registered && registeredCallback == OnPresent, "first Start must register callback");
        Start(LogB);
        Check(registrations == 1 && g_log == LogB, "second Start must retain registration and update logger");
        g_active = true;
        g_installed = true;
        Stop();
        Check(!g_registered && !g_active && !g_installed && !registeredCallback,
              "Stop must clear registration, active and installed state");
        Stop();
        Check(unregistrations == 1, "second Stop must not unregister twice");
        Start(LogA);
        Check(registrations == 2 && registeredCallback == OnPresent, "Start after Stop must register again");
        Stop();
        for (unsigned i = 0; i < 100; ++i) {
            Start(LogA);
            Start(LogB);
            Check(g_log == LogB, "restart must update logger");
            Stop();
            Stop();
        }
        Check(registrations == 102 && unregistrations == 102 && !registeredCallback,
              "repeated lifecycle must remain balanced");
        Check(detourRemovals == 204, "every Stop must preserve detour cleanup");
        std::cout << "PASS: production Start/Stop; 102 registrations balanced, duplicate calls safe, logger refreshed\n";
        return 0;
    } catch (const std::exception& error) {
        std::cerr << "FAIL: " << error.what() << '\n';
        return 1;
    }
}
'@
[IO.File]::WriteAllText((Join-Path $buildRoot 'present_test.cpp'), $harness)
[IO.File]::WriteAllText((Join-Path $buildRoot 'CMakeLists.txt'), @'
cmake_minimum_required(VERSION 3.20)
project(present_registration_test LANGUAGES CXX)
add_executable(present_registration_test present_test.cpp)
target_compile_features(present_registration_test PRIVATE cxx_std_17)
'@)

$cmakeCommand = Get-Command cmake -ErrorAction SilentlyContinue
if ($cmakeCommand) { $cmake = $cmakeCommand.Source }
else {
    $cmake = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin\cmake.exe'
    if (-not (Test-Path -LiteralPath $cmake)) { throw 'CMake not available; install Visual Studio 2022 C++ Build Tools.' }
}
& $cmake -S $buildRoot -B (Join-Path $buildRoot 'out') -G 'Visual Studio 17 2022' -A x64 2>&1 |
    Tee-Object -FilePath (Join-Path $buildRoot 'configure.log') | Out-Host
if ($LASTEXITCODE -ne 0) { throw "Configure failed; evidence: $buildRoot" }
& $cmake --build (Join-Path $buildRoot 'out') --config Release 2>&1 |
    Tee-Object -FilePath (Join-Path $buildRoot 'build.log') | Out-Host
if ($LASTEXITCODE -ne 0) { throw "Compile failed; evidence: $buildRoot" }
$previousErrorAction = $ErrorActionPreference
try {
    # Windows PowerShell 5 wraps native stderr as ErrorRecord; preserve expected
    # failing-fixture output and inspect the actual process exit code ourselves.
    $ErrorActionPreference = 'Continue'
    & (Join-Path $buildRoot 'out\Release\present_registration_test.exe') 2>&1 |
        Tee-Object -FilePath (Join-Path $buildRoot 'test.log') | Out-Host
    $testExit = $LASTEXITCODE
} finally { $ErrorActionPreference = $previousErrorAction }
Write-Host "Evidence: $buildRoot"
if ($testExit -ne 0) { throw "Production callback regression failed (exit $testExit)." }

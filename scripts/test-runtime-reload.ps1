[CmdletBinding()]
param([string]$Fork = '')
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path $PSScriptRoot -Parent
if (-not $Fork) { $Fork = Join-Path $repoRoot 'external\sk8v' }
$buildRoot = Join-Path $repoRoot ('build\qa-runtime-reload-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $buildRoot -Force | Out-Null
# Compile all production RuntimeBridge methods and its actual ABI declarations.
# Only Win32 DLL lookup is stubbed: no game/runtime DLL is loaded by the test.
foreach ($file in @('host\src\runtime_bridge.cpp', 'host\src\runtime_bridge.h', 'host\include\skatev_runtime.h')) {
    Copy-Item -LiteralPath (Join-Path $Fork $file) -Destination $buildRoot
}
$harness = @'
#include "runtime_bridge.h"
#include <cstring>
#include <iostream>
#include <stdexcept>
static unsigned creates = 0, destroys = 0, suspends = 0, loads = 0, lookups = 0;
static bool allowSuspend = true, allowCreate = true, allowModule = true;
static std::uint32_t abi = SKATEV_ABI_VERSION;
static const char* missing = nullptr;
static void* lastSuspended = nullptr;
static void* lastDestroyed = nullptr;
static std::uint32_t FakeApi() { return abi; }
static void* FakeCreate(const SvCreateInfo* info) {
    if (!info || info->size != sizeof(*info) || info->abi_version != SKATEV_ABI_VERSION)
        throw std::runtime_error("production create info invalid");
    ++creates;
    return allowCreate ? reinterpret_cast<void*>(static_cast<std::uintptr_t>(0x10000 + creates * 16)) : nullptr;
}
static void FakeDestroy(void* handle) { ++destroys; lastDestroyed = handle; }
static std::uint32_t FakeSuspend(void* handle) {
    ++suspends;
    lastSuspended = handle;
    return allowSuspend ? 1u : 0u;
}
static void UnusedExport() {}
static HMODULE FakeLoadLibraryW(const wchar_t*) {
    ++loads;
    return allowModule ? reinterpret_cast<HMODULE>(0x8000) : nullptr;
}
static DWORD FakeGetLastError() { return 126; }
static FARPROC FakeGetProcAddress(HMODULE, LPCSTR name) {
    ++lookups;
    if (missing && std::strcmp(name, missing) == 0) return nullptr;
    if (std::strcmp(name, "sv_api_version") == 0) return reinterpret_cast<FARPROC>(FakeApi);
    if (std::strcmp(name, "sv_create") == 0) return reinterpret_cast<FARPROC>(FakeCreate);
    if (std::strcmp(name, "sv_destroy") == 0) return reinterpret_cast<FARPROC>(FakeDestroy);
    if (std::strcmp(name, "sv_deactivate") == 0) return reinterpret_cast<FARPROC>(FakeSuspend);
    // Other exports are resolved but never called in these lifecycle tests.
    return reinterpret_cast<FARPROC>(UnusedExport);
}
#define LoadLibraryW FakeLoadLibraryW
#define GetProcAddress FakeGetProcAddress
#define GetLastError FakeGetLastError
#include "runtime_bridge.cpp"
#undef LoadLibraryW
#undef GetProcAddress
#undef GetLastError
static void Check(bool ok, const char* why) { if (!ok) throw std::runtime_error(why); }
static bool Load(RuntimeBridge& bridge, std::string& error) {
    return bridge.Load(L"fake-runtime.dll", "assets", "cache", "test.log", 3000, 3, error);
}
int main() {
    try {
        std::string error;
        RuntimeBridge bridge;
        Check(Load(bridge, error), "initial Load must succeed");
        void* const handle = bridge.Handle();
        void* const audioConsumer = handle;
        Check(handle && creates == 1 && destroys == 0 && suspends == 0, "initial Load must create once");
        const auto initialLookups = lookups;
        for (unsigned i = 0; i < 100; ++i) {
            error = "earlier diagnostic";
            Check(Load(bridge, error), "repeated Load must succeed");
            Check(error.empty(), "successful reuse must clear stale error");
            Check(bridge.Handle() == audioConsumer, "reload must retain the audio consumer's runtime identity");
        }
        Check(creates == 1 && destroys == 0 && suspends == 100, "reload must suspend, never orphan or destroy runtime");
        Check(lastSuspended == handle && loads == 1 && lookups == initialLookups,
              "reload must reuse exports/module without DLL churn");
        allowSuspend = false;
        error.clear();
        Check(!Load(bridge, error), "failed suspend enqueue must fail reload");
        Check(!error.empty() && bridge.Handle() == handle && creates == 1 && destroys == 0,
              "failed reload must report error and keep consumer handle alive");
        allowSuspend = true;
        Check(Load(bridge, error), "reload must recover after suspend failure");
        Check(error.empty(), "recovery must clear suspend failure diagnostic");
        bridge.Release();
        Check(!bridge.IsLoaded() && destroys == 1 && lastDestroyed == handle, "explicit Release must destroy once");
        bridge.Release();
        Check(destroys == 1, "repeated Release must not double destroy");
        Check(Load(bridge, error), "explicit Release followed by Load may create again");
        Check(creates == 2 && bridge.Handle() != handle, "fresh lifetime must create a new handle");
        bridge.Release();

        const auto beforeFailures = creates;
        RuntimeBridge missingExport;
        missing = "sv_step";
        error.clear();
        Check(!Load(missingExport, error) && !error.empty() && creates == beforeFailures,
              "missing required export must fail before creation");
        missing = nullptr;
        Check(Load(missingExport, error), "missing-export initial failure must permit retry");
        missingExport.Release();
        RuntimeBridge wrongAbi;
        abi = SKATEV_ABI_VERSION + 1;
        const auto beforeAbi = creates;
        error.clear();
        Check(!Load(wrongAbi, error) && !error.empty() && creates == beforeAbi,
              "wrong ABI must remain rejected");
        abi = SKATEV_ABI_VERSION;
        RuntimeBridge failedCreate;
        allowCreate = false;
        error.clear();
        Check(!Load(failedCreate, error) && !failedCreate.IsLoaded() && !error.empty(),
              "failed sv_create must remain rejected");
        allowCreate = true;
        Check(Load(failedCreate, error), "failed initial creation must permit retry");
        failedCreate.Release();
        RuntimeBridge failedModule;
        allowModule = false;
        error.clear();
        Check(!Load(failedModule, error) && !error.empty(), "missing DLL must remain rejected");
        std::cout << "PASS: actual RuntimeBridge; 100 reloads retain consumer handle, suspend failures safe, initial failure gates preserved\n";
        return 0;
    } catch (const std::exception& error) {
        std::cerr << "FAIL: " << error.what() << '\n';
        return 1;
    }
}
'@
[IO.File]::WriteAllText((Join-Path $buildRoot 'reload_test.cpp'), $harness)
# Extract the real HUD binding initializer, with only its platform dependencies
# mocked. This checks cache invalidation without pretending to render D3D.
$hudSourcePath = Join-Path $Fork 'host\src\hud_overlay.cpp'
Copy-Item -LiteralPath $hudSourcePath -Destination (Join-Path $buildRoot 'hud_overlay.source.cpp')
$hudSource = [IO.File]::ReadAllText($hudSourcePath)
$hudAt = $hudSource.IndexOf('void Start(const std::wstring& iniPath, void* runtime, LogFn log)', [StringComparison]::Ordinal)
if ($hudAt -lt 0) { throw 'HUD production Start signature not found.' }
$hudOpen = $hudSource.IndexOf('{', $hudAt)
$hudDepth = 1
for ($hudEnd = $hudOpen + 1; $hudEnd -lt $hudSource.Length; $hudEnd++) {
    if ($hudSource[$hudEnd] -eq '{') { $hudDepth++ }
    if ($hudSource[$hudEnd] -eq '}') { $hudDepth-- }
    if ($hudDepth -eq 0) { break }
}
if ($hudDepth -ne 0) { throw 'HUD production Start body unbalanced.' }
$hudStart = $hudSource.Substring($hudAt, $hudEnd - $hudAt + 1)
$hudHarness = @'
#include "skatev_runtime.h"
#include <windows.h>
#include <atomic>
#include <cstring>
#include <iostream>
#include <stdexcept>
#include <string>
using LogFn = void (*)(const char*);
static unsigned registrations = 0, rendererSetups = 0;
static bool enabledConfig = true, missingExports = false;
struct Shared {
    LogFn log = nullptr;
    bool enabled = false, registered = false;
    void* runtime = nullptr;
    SvSetHudViewportFn setViewport = nullptr;
    SvGetHudTextureFn getTexture = nullptr;
    SvGetHudFrameFn getFrame = nullptr;
    SvGetXrayFn getXray = nullptr;
    std::uint32_t (__cdecl* showRadio)(void*, const char*, const char*, const char*, const char*) = nullptr;
    SvHudViewport sent{};
    float maxAspect = 0, safeOverride = -1;
    std::atomic<bool> stopping{false};
} g;
namespace hudrender {
void SetLog(LogFn) { ++rendererSetups; }
void SetCompositeOverride(int) {}
}
namespace xrayrender { void SetLog(LogFn) {} }
template<class... Args> void Logf(const char*, Args...) {}
float IniFloat(const std::wstring&, const wchar_t* key, float value) {
    return std::wcscmp(key, L"Hud") == 0 && !enabledConfig ? 0.0f : value;
}
void OnPresent(void*) {}
void presentCallbackRegister(void (*)(void*)) {
    if (++registrations > 1) throw std::runtime_error("duplicate HUD callback");
}
static void UnusedExport() {}
static HMODULE FakeGetModuleHandleW(const wchar_t*) { return reinterpret_cast<HMODULE>(0x8000); }
static FARPROC FakeGetProcAddress(HMODULE, LPCSTR) {
    return missingExports ? nullptr : reinterpret_cast<FARPROC>(UnusedExport);
}
#define GetModuleHandleW FakeGetModuleHandleW
#define GetProcAddress FakeGetProcAddress
'@
$hudHarness += "`n$hudStart`n"
$hudHarness += @'
#undef GetModuleHandleW
#undef GetProcAddress
void LogA(const char*) {}
void LogB(const char*) {}
void Check(bool ok, const char* why) { if (!ok) throw std::runtime_error(why); }
int main() {
    try {
        auto* handle = reinterpret_cast<void*>(0x10000);
        Start(L"test.ini", handle, LogA);
        Check(g.enabled && g.runtime == handle && registrations == 1, "initial HUD binding failed");
        SvHudViewport viewport{};
        viewport.size = sizeof(viewport);
        viewport.width = 2560;
        viewport.height = 1440;
        for (unsigned i = 0; i < 100; ++i) {
            g.sent = viewport;
            Start(L"test.ini", handle, LogB);
            const SvHudViewport zero{};
            Check(std::memcmp(&g.sent, &zero, sizeof(zero)) == 0,
                  "same-handle script restart must invalidate sent viewport");
            Check(g.log == LogB && g.runtime == handle, "rebind must retain runtime and refresh logger");
            Check(std::memcmp(&g.sent, &viewport, sizeof(viewport)) != 0,
                  "initial viewport must differ so production Tick resends it");
        }
        Check(registrations == 1 && rendererSetups == 1, "restart must retain renderer/callback");
        enabledConfig = false;
        Start(L"test.ini", handle, LogA);
        Check(!g.enabled && registrations == 1, "disabled HUD must remain disabled");
        enabledConfig = true;
        missingExports = true;
        Start(L"test.ini", handle, LogA);
        Check(!g.enabled && registrations == 1, "missing HUD exports must remain inert");
        std::cout << "PASS: actual HUD Start; 100 same-handle restarts invalidate viewport without renderer/callback churn\n";
        return 0;
    } catch (const std::exception& error) {
        std::cerr << "FAIL: " << error.what() << '\n';
        return 1;
    }
}
'@
[IO.File]::WriteAllText((Join-Path $buildRoot 'hud_restart_test.cpp'), $hudHarness)
[IO.File]::WriteAllText((Join-Path $buildRoot 'CMakeLists.txt'), @'
cmake_minimum_required(VERSION 3.20)
project(runtime_reload_test LANGUAGES CXX)
add_executable(runtime_reload_test reload_test.cpp)
target_compile_features(runtime_reload_test PRIVATE cxx_std_17)
target_compile_definitions(runtime_reload_test PRIVATE NOMINMAX)
add_executable(hud_restart_test hud_restart_test.cpp)
target_compile_features(hud_restart_test PRIVATE cxx_std_17)
target_compile_definitions(hud_restart_test PRIVATE NOMINMAX)
'@)
$command = Get-Command cmake -ErrorAction SilentlyContinue
if ($command) { $cmake = $command.Source }
else { $cmake = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin\cmake.exe' }
if (-not (Test-Path -LiteralPath $cmake)) { throw 'CMake / Visual Studio C++ Build Tools unavailable.' }
& $cmake -S $buildRoot -B (Join-Path $buildRoot 'out') -G 'Visual Studio 17 2022' -A x64 2>&1 |
    Tee-Object -FilePath (Join-Path $buildRoot 'configure.log') | Out-Host
if ($LASTEXITCODE -ne 0) { throw "Configure failed; evidence: $buildRoot" }
& $cmake --build (Join-Path $buildRoot 'out') --config Release 2>&1 |
    Tee-Object -FilePath (Join-Path $buildRoot 'build.log') | Out-Host
if ($LASTEXITCODE -ne 0) { throw "Compile failed; evidence: $buildRoot" }
$priorErrorAction = $ErrorActionPreference
try {
    $ErrorActionPreference = 'Continue'
    $testExit = 0
    foreach ($testName in @('runtime_reload_test', 'hud_restart_test')) {
        & (Join-Path $buildRoot "out\Release\$testName.exe") 2>&1 |
            Tee-Object -FilePath (Join-Path $buildRoot "$testName.log") | Out-Host
        if ($LASTEXITCODE -ne 0) { $testExit = $LASTEXITCODE }
    }
} finally { $ErrorActionPreference = $priorErrorAction }
Write-Host "Evidence: $buildRoot"
if ($testExit -ne 0) { throw "Runtime reload regression failed (exit $testExit)." }

using System;
using GTAxSkate.Coop;

internal static class SkateSessionTests
{
    private static int checks;
    private static void Check(bool condition, string message)
    {
        checks++;
        if (!condition) throw new Exception("Check failed: " + message);
    }

    public static int Main()
    {
        try
        {
            Run();
            Console.WriteLine("PASS: " + checks + " session & RAGECOOP slot-mapping checks; no live server/game tested.");
            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine(ex.Message);
            return 1;
        }
    }

    private static void Run()
    {
        // 1. Signed nonzero Int32 -> strictly positive unique slot mapping (RAGECOOP contract).
        var map = new PeerSlotMap();
        int s0, sNeg, sPos, sNegRepeat;
        Check(!map.TryAssignOrGetSlot(0, out s0) && s0 == 0, "reject raw id 0");
        Check(map.TryAssignOrGetSlot(-5, out sNeg) && sNeg > 0, "negative raw id maps positive");
        Check(map.TryAssignOrGetSlot(5, out sPos) && sPos > 0 && sPos != sNeg, "-5 and +5 must not collide");
        Check(map.TryAssignOrGetSlot(-5, out sNegRepeat) && sNegRepeat == sNeg, "idempotent slot for existing raw id");

        int resolvedRaw;
        Check(map.TryGetRawId(sNeg, out resolvedRaw) && resolvedRaw == -5, "reverse lookup raw id");

        int released;
        Check(map.TryReleaseByRawId(-5, out released) && released == sNeg, "release raw -5");
        Check(!map.TryGetSlot(-5, out sNegRepeat), "released id gone");

        // Monotonicity on reconnect + max 64 capacity enforcement.
        int sNegReconnect;
        Check(map.TryAssignOrGetSlot(-5, out sNegReconnect) && sNegReconnect > sPos, "reassigned slot stays monotonic");
        for (int i = 100; map.Count < PeerSlotMap.MaxPeers; i++)
        {
            int tmp;
            Check(map.TryAssignOrGetSlot(i, out tmp) && tmp > 0, "fill up to 64 peers");
        }
        int overflow;
        Check(!map.TryAssignOrGetSlot(999999, out overflow), "reject 65th peer");

        // Interop with ServerPoseSessions using mapped positive slots (never raw negative IDs).
        var poseGuard = new ServerPoseSessions();
        var dummyLayout = new SkeletonLayout(new PoseFrame(
            Guid.NewGuid(), 1, 777, false, Matrix12.Identity, Matrix12.Identity,
            new[] { new BonePose(0, -1, Matrix12.Identity) },
            new[]
            {
                Matrix12.Identity, Matrix12.Identity, Matrix12.Identity, Matrix12.Identity,
                Matrix12.Identity, Matrix12.Identity, Matrix12.Identity
            }));
        Guid nonce = poseGuard.BeginAuthenticatedSession(sNegReconnect, dummyLayout, 100);
        Check(nonce != Guid.Empty, "mapped positive slot opens ServerPoseSessions cleanly");

        // 2. Skate 3-style Host/Join session & Freeskate Activity coordinator.
        var sessions = new SkateSessionCoordinator();
        string code;
        Check(!sessions.TryCreateSession(0, SkateWorldZone.LosSantos, SkateMode.FreeSkate, "bad", 100, out code), "reject slot 0 create");
        Check(sessions.TryCreateSession(sPos, SkateWorldZone.SkateSchoolIsland, SkateMode.FreeSkate, "Coach Frank Lab\n\r", 100, out code), "host session on Skate School Island");
        Check(code != null && code.Length == 6, "6-char join code");
        Check(sessions.TryJoinByCode(sNegReconnect, code.ToLowerInvariant(), 120), "join by case-insensitive code");
        Check(!sessions.TryJoinByCode(sNegReconnect, code, 120), "prevent duplicate join");

        // Non-host cannot reconfigure or start activities.
        Check(!sessions.TryConfigureActivity(sNegReconnect, SkateMode.SpotBattle, SkateWorldZone.SkateSchoolIsland, "sksc_plaza", 60000), "non-host configure blocked");
        Check(!sessions.TryStartActivity(sNegReconnect, 130), "non-host start blocked");

        // Host starts a 1-minute Spot Battle on Skate School Island.
        Check(sessions.TryConfigureActivity(sPos, SkateMode.SpotBattle, SkateWorldZone.SkateSchoolIsland, "sksc_plaza", 60000), "host configures SpotBattle");
        Check(sessions.TryStartActivity(sPos, 200), "host starts SpotBattle");
        Check(!sessions.TryConfigureActivity(sPos, SkateMode.SKATE, SkateWorldZone.LosSantos, "vespucci", 60000), "cannot reconfigure mid-activity");

        // Score guards: reject underwater or negative score; rank leaderboard cleanly.
        Check(!sessions.TryReportScore(sPos, 1500, 1500, false, 300), "reject underwater score report");
        Check(!sessions.TryReportScore(sPos, -10, 0, true, 300), "reject negative score");
        Check(sessions.TryReportScore(sPos, 1200, 1200, true, 300), "accept host dry-land score");
        Check(sessions.TryReportScore(sNegReconnect, 2850, 2850, true, 350), "accept peer dry-land score");
        Check(sessions.TryUpdateBoardProfile(sNegReconnect, "deck=flip_black;wheels=spitfire_52"), "sync board profile string");

        SkateSessionSnapshot snap;
        Check(sessions.TryGetSnapshotForPeer(sPos, 400, out snap), "fetch snapshot");
        Check(snap.Phase == ActivityPhase.Active && snap.Zone == SkateWorldZone.SkateSchoolIsland, "snapshot state matches");
        Check(snap.Peers.Length == 2 && snap.Peers[0].Slot == sNegReconnect && snap.Peers[0].RoundScore == 2850, "leaderboard sorted descending");

        // Activity expires deterministically once deadline reached.
        Check(!sessions.TryReportScore(sPos, 5000, 5000, true, 60201), "reject score past duration deadline");
        Check(sessions.TryGetSnapshotForPeer(sPos, 60201, out snap) && snap.Phase == ActivityPhase.Finished, "phase transitions to Finished");

        // Host disconnect migrates host cleanly to remaining peer, then closes when empty.
        bool closed;
        int newHost;
        Check(sessions.TryLeaveOrDisconnect(sPos, out closed, out newHost) && !closed && newHost == sNegReconnect, "host migrates on host disconnect");
        Check(sessions.TryLeaveOrDisconnect(sNegReconnect, out closed, out newHost) && closed && newHost == 0 && sessions.SessionCount == 0, "session closes cleanly when empty");

        // Enforce max 10 players per session (Skate 3 player cap).
        string capCode;
        Check(sessions.TryCreateSession(1, SkateWorldZone.StartParkIsland, SkateMode.OneUp, "StartPark Jam", 1000, out capCode), "create cap session");
        for (int p = 2; p <= SkateSessionCoordinator.MaxPlayersPerSession; p++)
            Check(sessions.TryJoinByCode(p, capCode, 1000 + p), "allow up to 10 skaters");
        Check(!sessions.TryJoinByCode(11, capCode, 1020), "reject 11th skater");

        // 3. Friend-Coop Security Policy & Always-Visible Connection Awareness Tests.
        string friendKey = "SkateFriends-2026-Key!";

        // Server config fail-closed checks: public announce or random join rejected.
        Check(!CoopSecurityPolicy.ValidateServerConfig(new ServerSecurityConfig(true, false, true, friendKey, true), "127.0.0.1").Allowed,
            "reject AnnounceSelf=true public listing");
        Check(!CoopSecurityPolicy.ValidateServerConfig(new ServerSecurityConfig(false, true, true, friendKey, true), "127.0.0.1").Allowed,
            "reject random public join server");
        Check(!CoopSecurityPolicy.ValidateServerConfig(ServerSecurityConfig.DefaultPrivateFriends("short"), "127.0.0.1").Allowed,
            "reject short weak friend key");
        Check(!CoopSecurityPolicy.ValidateServerConfig(ServerSecurityConfig.DefaultPrivateFriends(friendKey), "203.0.113.5").Allowed,
            "reject raw public IP bind when private-only enforced");
        Check(CoopSecurityPolicy.ValidateServerConfig(ServerSecurityConfig.DefaultPrivateFriends(friendKey), "100.88.12.34").Allowed,
            "allow Tailscale VPN bind for private friend server");

        // Endpoint trust classification checks.
        Check(CoopSecurityPolicy.ClassifyEndpoint("127.0.0.1") == EndpointTrustClass.Loopback, "IPv4 loopback");
        Check(CoopSecurityPolicy.ClassifyEndpoint("192.168.1.42") == EndpointTrustClass.PrivateLan, "IPv4 LAN");
        Check(CoopSecurityPolicy.ClassifyEndpoint("10.0.0.9") == EndpointTrustClass.PrivateLan, "IPv4 10/8 LAN");
        Check(CoopSecurityPolicy.ClassifyEndpoint("100.101.55.7") == EndpointTrustClass.PrivateVpnOverlay, "Tailscale CGNAT VPN");
        Check(CoopSecurityPolicy.ClassifyEndpoint("fc00::1234") == EndpointTrustClass.PrivateVpnOverlay, "IPv6 ULA VPN");
        Check(CoopSecurityPolicy.ClassifyEndpoint("198.51.100.25") == EndpointTrustClass.PublicInternet, "public IP classified");

        // Random online lobbies & unpinned public hosts must be blocked.
        var tracker = new SessionConnectionTracker();
        Check(!tracker.IsConnectedToPlayerSession && tracker.BuildStatusBanner().Contains("[SOLO]"), "starts clearly in Solo");

        byte[] cNonce = Guid.NewGuid().ToByteArray();
        byte[] sNonce = Guid.NewGuid().ToByteArray();
        PolicyDecision decision;

        Check(!tracker.BeginFriendConnect(
            new ClientConnectRequest(LobbyDiscoverySource.PublicServerBrowser, "192.168.1.20", 4499, true, "RandomServer", friendKey),
            cNonce, 100, out decision) && !decision.Allowed, "block public server browser");
        Check(tracker.Status == ConnectionSecurityStatus.BlockedInsecure && tracker.BuildStatusBanner().Contains("BLOCKED"),
            "tracker explicitly shows BLOCKED banner");

        Check(!tracker.BeginFriendConnect(
            new ClientConnectRequest(LobbyDiscoverySource.RandomMatchmaking, "100.64.0.5", 4499, true, "QuickLobby", friendKey),
            cNonce, 110, out decision), "block random matchmaking");

        Check(!tracker.BeginFriendConnect(
            new ClientConnectRequest(LobbyDiscoverySource.DirectFriendInvite, "198.51.100.25", 4499, false, "StrangerPublic", friendKey),
            cNonce, 120, out decision), "block unpinned public internet endpoint");

        // Valid VPN friend invite starts verification and rejects spoofed host proof.
        Check(tracker.BeginFriendConnect(
            new ClientConnectRequest(LobbyDiscoverySource.DirectFriendInvite, "100.88.12.34", 4499, false, "VPN host", friendKey),
            cNonce, 130, out decision) && decision.Allowed && decision.TrustClass == EndpointTrustClass.PrivateVpnOverlay,
            "permit Tailscale friend connect into Verifying state");
        Check(tracker.Status == ConnectionSecurityStatus.VerifyingFriendSession && tracker.BuildStatusBanner().Contains("[CHECKING]"),
            "explicit Checking status while handshaking");

        byte[] wrongProof = CoopSecurityPolicy.ComputeMutualProof("WrongFriendPassword!", cNonce, sNonce, "SRV");
        Check(!tracker.CompleteMutualServerVerification(cNonce, sNonce, wrongProof, "SK8ABC", 2, 140),
            "reject spoofed server with wrong friend key");
        Check(tracker.Status == ConnectionSecurityStatus.BlockedInsecure && !tracker.IsConnectedToPlayerSession,
            "spoofed server drops fail-closed to BlockedInsecure");

        // Re-initiate valid friend session and complete genuine mutual HMAC handshake.
        Check(tracker.BeginFriendConnect(
            new ClientConnectRequest(LobbyDiscoverySource.DirectFriendInvite, "100.88.12.34", 4499, false, "VPN host", friendKey),
            cNonce, 150, out decision), "start valid VPN friend connect");
        byte[] validClientProof = CoopSecurityPolicy.ComputeMutualProof(friendKey, cNonce, sNonce, "CLI");
        byte[] validServerProof = CoopSecurityPolicy.ComputeMutualProof(friendKey, cNonce, sNonce, "SRV");
        Check(CoopSecurityPolicy.VerifyMutualProof(friendKey, cNonce, sNonce, "CLI", validClientProof), "client proof validates on server");
        Check(!CoopSecurityPolicy.VerifyMutualProof(friendKey, cNonce, sNonce, "SRV", validClientProof), "role separation prevents reflection attack");

        Check(tracker.CompleteMutualServerVerification(cNonce, sNonce, validServerProof, "SK8ABC", 2, 160),
            "complete genuine mutual friend handshake");
        Check(tracker.IsConnectedToPlayerSession && tracker.BuildStatusBanner().Contains("[CONNECTED - PRIVATE COOP]") &&
            tracker.BuildStatusBanner().Contains("VPN host") && tracker.BuildStatusBanner().Contains("[VPN]") &&
            !tracker.BuildStatusBanner().Contains("SK8ABC") && !tracker.BuildStatusBanner().Contains("100.88.12.34") &&
            !tracker.BuildStatusBanner().Contains("Code"), "shows the connected friend host label + trust tag, never the join code or the address");

        tracker.UpdateLivePeerCount(4, 170);
        Check(tracker.BuildStatusBanner().Contains("Skaters: 4"), "banner reflects live friend count");
        tracker.DisconnectToSolo("Returned to Solo Story", 180);
        Check(!tracker.IsConnectedToPlayerSession && tracker.Status == ConnectionSecurityStatus.OfflineSolo, "clean disconnect returns to Solo");

        // 6. Mod-parity & interference warning checker: ONLY genuine coop-interfering mods trigger warnings.
        var leakyPath = new ModEntry(@"C:\Users\PrivateName\GTA5\scripts\TrafficOverhaul.dll", "AB12cd",
            ModCompatibilityChecker.ClassifyModPath(@"C:\Users\PrivateName\GTA5\scripts\TrafficOverhaul.dll"));
        Check(!leakyPath.RelativeId.Contains("PrivateName") && !leakyPath.RelativeId.Contains("C:") &&
            leakyPath.RelativeId == "TrafficOverhaul.dll" && leakyPath.Digest == "ab12cd" &&
            leakyPath.Impact == ModImpactClass.GameplayInterference,
            "mod id sanitizer strips drive letters/usernames and classifies traffic mod as GameplayInterference");
        Check(ModCompatibilityChecker.ClassifyModPath("mods/reshade-shaders/ReShade.ini") == ModImpactClass.CosmeticLocalOnly &&
            ModCompatibilityChecker.ClassifyModPath("dxgi.dll") == ModImpactClass.CosmeticLocalOnly &&
            ModCompatibilityChecker.ClassifyModPath("HeapAdjuster.asi") == ModImpactClass.CosmeticLocalOnly &&
            ModCompatibilityChecker.ClassifyModPath("PackfileLimitAdjusterEnhanced.asi") == ModImpactClass.CosmeticLocalOnly &&
            ModCompatibilityChecker.ClassifyModPath("Menyoo.asi") == ModImpactClass.CosmeticLocalOnly &&
            ModCompatibilityChecker.ClassifyModPath("TrainerV.asi") == ModImpactClass.CosmeticLocalOnly &&
            ModCompatibilityChecker.ClassifyModPath("shadows.asi") == ModImpactClass.CosmeticLocalOnly &&
            ModCompatibilityChecker.ClassifyModPath("scripts/NativeUI.dll") == ModImpactClass.CosmeticLocalOnly &&
            ModCompatibilityChecker.ClassifyModPath("scripts/SimpleSpeedometer.dll") == ModImpactClass.CosmeticLocalOnly &&
            ModCompatibilityChecker.ClassifyModPath("dlcpacks/rudenudeloading/dlc.rpf") == ModImpactClass.CosmeticLocalOnly,
            "harmless graphics, infra, trainers, UI, and unknown benign scripts never trigger warnings");
        Check(ModCompatibilityChecker.ClassifyModPath("dlcpacks/skate_school/dlc.rpf") == ModImpactClass.CriticalMapOrPhysics &&
            ModCompatibilityChecker.ClassifyModPath("SkateVRuntime.dll") == ModImpactClass.CriticalMapOrPhysics &&
            ModCompatibilityChecker.ClassifyModPath("update/handling.meta") == ModImpactClass.CriticalMapOrPhysics,
            "SK8V/map/handling classified as CriticalMapOrPhysics");

        ModEntry[] localMods = new[]
        {
            new ModEntry("dlcpacks/skate_school/dlc.rpf", "112233", ModImpactClass.CriticalMapOrPhysics),
            new ModEntry("SkateVRuntime.dll", "aabbcc", ModImpactClass.CriticalMapOrPhysics),
            new ModEntry("dxgi.dll", "local-enb", ModImpactClass.CosmeticLocalOnly),
            new ModEntry("scripts/SimpleSpeedometer.dll", "1.0", ModCompatibilityChecker.ClassifyModPath("scripts/SimpleSpeedometer.dll"))
        };
        ModEntry[] matchingRemote = new[]
        {
            new ModEntry("dlcpacks/skate_school/dlc.rpf", "112233", ModImpactClass.CriticalMapOrPhysics),
            new ModEntry("SkateVRuntime.dll", "aabbcc", ModImpactClass.CriticalMapOrPhysics),
            new ModEntry("reshade.ini", "different-preset", ModImpactClass.CosmeticLocalOnly),
            new ModEntry("Menyoo.asi", "9.9", ModCompatibilityChecker.ClassifyModPath("Menyoo.asi")),
            new ModEntry("HeapAdjuster.asi", "1.0", ModCompatibilityChecker.ClassifyModPath("HeapAdjuster.asi"))
        };
        ModCompatibilityReport cleanMods = ModCompatibilityChecker.CompareManifests(localMods, matchingRemote, "FriendA");
        Check(!cleanMods.HasWarnings, "matching coop mods with different ENB/ReShade/trainers/adjusters produce zero warnings");

        ModEntry[] conflictingRemote = new[]
        {
            new ModEntry("SkateVRuntime.dll", "ff0099", ModImpactClass.CriticalMapOrPhysics),
            new ModEntry("scripts/CrazyTraffic.dll", "100", ModImpactClass.GameplayInterference)
        };
        ModCompatibilityReport mismatch = ModCompatibilityChecker.CompareManifests(localMods, conflictingRemote, "FriendB");
        Check(mismatch.HasWarnings && mismatch.Mismatches.Length == 3 &&
            mismatch.Mismatches[0].Impact == ModImpactClass.CriticalMapOrPhysics &&
            mismatch.WarningSummary.Contains("HIGH DESYNC RISK") &&
            mismatch.WarningSummary.Contains("MAY INTERFERE") &&
            mismatch.WarningSummary.Contains("dlcpacks/skate_school/dlc.rpf") &&
            mismatch.WarningSummary.Contains("CrazyTraffic.dll") &&
            !mismatch.WarningSummary.Contains("dxgi.dll") &&
            !mismatch.WarningSummary.Contains("SimpleSpeedometer.dll"),
            "mod mismatch flags ONLY interfering mods and ignores benign local mods");

        // 7. LocalModScanner hashes a mock GTA root without exposing temp path/username and ignores harmless local files.
        string tempRootA = System.IO.Path.Combine(System.IO.Path.GetTempPath(), "sk8_scan_a_" + Guid.NewGuid().ToString("N"));
        string tempRootB = System.IO.Path.Combine(System.IO.Path.GetTempPath(), "sk8_scan_b_" + Guid.NewGuid().ToString("N"));
        try
        {
            System.IO.Directory.CreateDirectory(System.IO.Path.Combine(tempRootA, "SK8V", "tutorial"));
            System.IO.Directory.CreateDirectory(System.IO.Path.Combine(tempRootA, "scripts"));
            System.IO.Directory.CreateDirectory(System.IO.Path.Combine(tempRootB, "scripts"));
            System.IO.File.WriteAllText(System.IO.Path.Combine(tempRootA, "SkateVRuntime.dll"), "runtime-v1");
            System.IO.File.WriteAllText(System.IO.Path.Combine(tempRootA, "SK8V", "tutorial", "m0-tutorial-manifest.json"), "{v1}");
            System.IO.File.WriteAllText(System.IO.Path.Combine(tempRootA, "HeapAdjuster.asi"), "harmless-infra");
            System.IO.File.WriteAllText(System.IO.Path.Combine(tempRootA, "ReShade.ini"), "presetA");
            System.IO.File.WriteAllText(System.IO.Path.Combine(tempRootA, "scripts", "RageCoop.Client.dll"), "skip-self");
            System.IO.File.WriteAllText(System.IO.Path.Combine(tempRootA, "scripts", "NativeUI.dll"), "benign-ui");

            System.IO.File.WriteAllText(System.IO.Path.Combine(tempRootB, "SkateVRuntime.dll"), "runtime-v1");
            System.IO.File.WriteAllText(System.IO.Path.Combine(tempRootB, "Menyoo.asi"), "sp-trainer");
            System.IO.File.WriteAllText(System.IO.Path.Combine(tempRootB, "scripts", "ExtraTraffic.dll"), "chaos");
            System.IO.File.WriteAllText(System.IO.Path.Combine(tempRootB, "dxgi.dll"), "presetB");

            ModEntry[] scanA = LocalModScanner.ScanGtaRoot(tempRootA);
            ModEntry[] scanB = LocalModScanner.ScanGtaRoot(tempRootB);
            Check(scanA.Length == 2 && scanB.Length == 2, "scanner retains ONLY runtime/optional/interfering mods and filters benign ASIs/UI/graphics");
            ModCompatibilityReport scannedReport = ModCompatibilityChecker.CompareManifests(scanB, scanA, "FriendOpt");
            Check(scannedReport.HasWarnings &&
                scannedReport.WarningSummary.Contains("m0-tutorial-manifest.json") &&
                scannedReport.WarningSummary.Contains("HIGH DESYNC RISK") &&
                scannedReport.WarningSummary.Contains("ExtraTraffic.dll") &&
                !scannedReport.WarningSummary.Contains(tempRootA) &&
                !scannedReport.WarningSummary.Contains("HeapAdjuster") &&
                !scannedReport.WarningSummary.Contains("Menyoo") &&
                !scannedReport.WarningSummary.Contains("NativeUI") &&
                !scannedReport.WarningSummary.Contains("ReShade.ini"),
                "scanned mod parity flags ONLY interfering mods without false positives or host paths");

            // Same interfering mods on both friends (even alongside different harmless local mods) -> zero warnings.
            string tempRootC = System.IO.Path.Combine(System.IO.Path.GetTempPath(), "sk8_scan_c_" + Guid.NewGuid().ToString("N"));
            try
            {
                System.IO.Directory.CreateDirectory(System.IO.Path.Combine(tempRootA, "dlcpacks", "addonpeds"));
                System.IO.File.WriteAllText(System.IO.Path.Combine(tempRootA, "scripts", "RealismTraffic.dll"), "traffic-v2");
                System.IO.File.WriteAllText(System.IO.Path.Combine(tempRootA, "dlcpacks", "addonpeds", "dlc.rpf"), "peds-v1");

                System.IO.Directory.CreateDirectory(System.IO.Path.Combine(tempRootC, "SK8V", "tutorial"));
                System.IO.Directory.CreateDirectory(System.IO.Path.Combine(tempRootC, "scripts"));
                System.IO.Directory.CreateDirectory(System.IO.Path.Combine(tempRootC, "dlcpacks", "addonpeds"));
                System.IO.File.WriteAllText(System.IO.Path.Combine(tempRootC, "SkateVRuntime.dll"), "runtime-v1");
                System.IO.File.WriteAllText(System.IO.Path.Combine(tempRootC, "SK8V", "tutorial", "m0-tutorial-manifest.json"), "{v1}");
                System.IO.File.WriteAllText(System.IO.Path.Combine(tempRootC, "scripts", "RealismTraffic.dll"), "traffic-v2");
                System.IO.File.WriteAllText(System.IO.Path.Combine(tempRootC, "dlcpacks", "addonpeds", "dlc.rpf"), "peds-v1");
                System.IO.File.WriteAllText(System.IO.Path.Combine(tempRootC, "Menyoo.asi"), "different-trainer");
                System.IO.File.WriteAllText(System.IO.Path.Combine(tempRootC, "TrainerV.asi"), "extra-trainer");
                System.IO.File.WriteAllText(System.IO.Path.Combine(tempRootC, "dxgi.dll"), "different-shader");

                ModEntry[] scanSameA = LocalModScanner.ScanGtaRoot(tempRootA);
                ModEntry[] scanSameC = LocalModScanner.ScanGtaRoot(tempRootC);
                ModCompatibilityReport sameReport = ModCompatibilityChecker.CompareManifests(scanSameA, scanSameC, "FriendSame");
                Check(!sameReport.HasWarnings && sameReport.Mismatches.Length == 0,
                    "identical interfering mods on both friends are ignored completely");
            }
            finally
            {
                try { if (System.IO.Directory.Exists(tempRootC)) System.IO.Directory.Delete(tempRootC, true); } catch { }
            }
        }
        finally
        {
            try { if (System.IO.Directory.Exists(tempRootA)) System.IO.Directory.Delete(tempRootA, true); } catch { }
            try { if (System.IO.Directory.Exists(tempRootB)) System.IO.Directory.Delete(tempRootB, true); } catch { }
        }

        // 7b. Skate School optional tutorial expansion parity (install.ps1 -WithTutorial layout).
        string tempRootSchoolA = System.IO.Path.Combine(System.IO.Path.GetTempPath(), "sk8_scan_sch_a_" + Guid.NewGuid().ToString("N"));
        string tempRootSchoolB = System.IO.Path.Combine(System.IO.Path.GetTempPath(), "sk8_scan_sch_b_" + Guid.NewGuid().ToString("N"));
        try
        {
            System.IO.Directory.CreateDirectory(System.IO.Path.Combine(tempRootSchoolA, "SK8V", "addons", "skate_school", "collision"));
            System.IO.Directory.CreateDirectory(System.IO.Path.Combine(tempRootSchoolA, "dlcpacks", "skate_school"));
            System.IO.Directory.CreateDirectory(tempRootSchoolB);

            System.IO.File.WriteAllText(System.IO.Path.Combine(tempRootSchoolA, "SkateVRuntime.dll"), "runtime-v1");
            System.IO.File.WriteAllText(System.IO.Path.Combine(tempRootSchoolB, "SkateVRuntime.dll"), "runtime-v1");

            System.IO.File.WriteAllText(
                System.IO.Path.Combine(tempRootSchoolA, "SK8V", "addons", "skate_school", "collision", "DIST_SkateSchool.svwc"),
                "collision-binary-mock");
            System.IO.File.WriteAllText(
                System.IO.Path.Combine(tempRootSchoolA, "dlcpacks", "skate_school", "dlc.rpf"),
                "dlc-rpf-binary-mock");

            ModEntry[] scanSchA = LocalModScanner.ScanGtaRoot(tempRootSchoolA);
            ModEntry[] scanSchB = LocalModScanner.ScanGtaRoot(tempRootSchoolB);

            ModCompatibilityReport diffReport = ModCompatibilityChecker.CompareManifests(scanSchB, scanSchA, "FriendSchool");
            Check(diffReport.HasWarnings &&
                diffReport.WarningSummary.Contains("DIST_SkateSchool.svwc") &&
                diffReport.WarningSummary.Contains("dlcpacks/skate_school/dlc.rpf") &&
                diffReport.WarningSummary.Contains("HIGH DESYNC RISK"),
                "skate_school collision + root dlc.rpf flags HIGH DESYNC RISK against peer without it");

            // Matching files on both peers produce ZERO warnings.
            System.IO.Directory.CreateDirectory(System.IO.Path.Combine(tempRootSchoolB, "SK8V", "addons", "skate_school", "collision"));
            System.IO.Directory.CreateDirectory(System.IO.Path.Combine(tempRootSchoolB, "dlcpacks", "skate_school"));
            System.IO.File.WriteAllText(
                System.IO.Path.Combine(tempRootSchoolB, "SK8V", "addons", "skate_school", "collision", "DIST_SkateSchool.svwc"),
                "collision-binary-mock");
            System.IO.File.WriteAllText(
                System.IO.Path.Combine(tempRootSchoolB, "dlcpacks", "skate_school", "dlc.rpf"),
                "dlc-rpf-binary-mock");

            ModEntry[] scanSchB2 = LocalModScanner.ScanGtaRoot(tempRootSchoolB);
            ModCompatibilityReport matchReport = ModCompatibilityChecker.CompareManifests(scanSchA, scanSchB2, "FriendSchoolMatch");
            Check(!matchReport.HasWarnings && matchReport.Mismatches.Length == 0,
                "skate_school collision + root dlc.rpf produces ZERO warnings when both peers match");

            // Long DLC pack names (>64 chars) keep CriticalMapOrPhysics and distinct prefixes; cosmetic names avoid false positives.
            string longPackA = "dlcpacks/very_long_skate_custom_park_pack_alpha_0123456789abcdef_end/dlc.rpf";
            string longPackB = "dlcpacks/very_long_skate_custom_park_pack_bravo_0123456789abcdef_end/dlc.rpf";
            string sanitizedLongA = ModEntry.SanitizeRelativeId(longPackA);
            string sanitizedLongB = ModEntry.SanitizeRelativeId(longPackB);
            Check(sanitizedLongA.Length <= 64 && sanitizedLongB.Length <= 64 && sanitizedLongA != sanitizedLongB,
                "long DLC relative IDs stay <= 64 chars without colliding");
            Check(ModCompatibilityChecker.ClassifyModPath(longPackA) == ModImpactClass.CriticalMapOrPhysics,
                "long DLC relative ID retains CriticalMapOrPhysics classification");
            Check(ModCompatibilityChecker.ClassifyModPath("scripts/skate_school_camera_preset.cs") == ModImpactClass.CosmeticLocalOnly,
                "unrelated file merely containing skate_school is not flagged critical");

            // Conflicting mods/update/x64/dlcpacks vs root dlcpacks copies with different content are not hidden.
            System.IO.Directory.CreateDirectory(System.IO.Path.Combine(tempRootSchoolA, "mods", "update", "x64", "dlcpacks", "skate_school"));
            System.IO.File.WriteAllText(
                System.IO.Path.Combine(tempRootSchoolA, "mods", "update", "x64", "dlcpacks", "skate_school", "dlc.rpf"),
                "mods-copy-v2-different");
            ModEntry[] scanConflict = LocalModScanner.ScanGtaRoot(tempRootSchoolA);
            bool hasRootConflictEntry = false;
            for (int idx = 0; idx < scanConflict.Length; idx++)
            {
                if (scanConflict[idx].RelativeId == "root-dlcpacks/skate_school/dlc.rpf")
                    hasRootConflictEntry = true;
            }
            Check(hasRootConflictEntry, "differing mods vs root dlc.rpf copies emit separate root-dlcpacks conflict entry");
        }
        finally
        {
            try { if (System.IO.Directory.Exists(tempRootSchoolA)) System.IO.Directory.Delete(tempRootSchoolA, true); } catch { }
            try { if (System.IO.Directory.Exists(tempRootSchoolB)) System.IO.Directory.Delete(tempRootSchoolB, true); } catch { }
        }

        // 8. ClientConfigLoader accepts hardened VPN config and rejects tampered public endpoints.
        ClientConnectRequest cfgReq;
        string cfgErr;
        string validJson = "{\n  \"BlockPublicServerBrowser\": true,\n  \"TrustedHostLabel\": \"Alex-VPN\",\n  \"TrustedHostIp\": \"100.88.12.34\",\n  \"Port\": 4499,\n  \"FriendKey\": \"SkateFriends-Secret-99\"\n}";
        Check(ClientConfigLoader.TryParseClientConfigJson(validJson, out cfgReq, out cfgErr) &&
            cfgReq != null && cfgReq.EndpointAddress == "100.88.12.34" && cfgReq.Port == 4499 && cfgReq.HostLabel == "Friend",
            "loads valid hardened VPN client config");

        string tamperedPublic = validJson.Replace("100.88.12.34", "203.0.113.5");
        Check(!ClientConfigLoader.TryParseClientConfigJson(tamperedPublic, out cfgReq, out cfgErr),
            "rejects hand-edited unpinned public IP in config");
        string tamperedBrowser = validJson.Replace("\"BlockPublicServerBrowser\": true", "\"BlockPublicServerBrowser\": false");
        Check(!ClientConfigLoader.TryParseClientConfigJson(tamperedBrowser, out cfgReq, out cfgErr),
            "rejects hand-edited BlockPublicServerBrowser=false in config");
    }
}

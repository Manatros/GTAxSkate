-- Loaded inside the staged SK8M resource so Config/Tune and the stop callback
-- have an explicit, versioned seam. No memory hooks or private assets.
Friends = { version = 1, stop = function() end }
local snapshot, sequence = nil, 0
-- A fresh resource lifetime must not accept a delayed reply to an old counter.
local nonce = tostring(GetGameTimer()) .. ':' .. tostring(math.random(1, 2147483647))
local syncRemaining, syncAt = 15, 0
local baseline = { PushMax = Config.PushMax, PushImpulse = Config.PushImpulse,
    BrakeDecel = Config.BrakeDecel, TurnRate = Config.TurnRate }
local previousGate = Config.CanRide
local pending, reply, grant
local function same(a, b)
    return a and b and a.generation == b.generation and a.revision == b.revision
end
function Friends.canRide()
    return snapshot ~= nil and GetResourceState('gtaxskate_rules') == 'started' and
        GetResourceState('gtaxskate_sk8m') == 'started'
end
function Friends.invalidate()
    snapshot, pending, reply, grant = nil, nil, nil, nil
    Friends.stop()
end
Config.CanRide = function(ped)
    if not Friends.canRide() then return false end
    if previousGate then
        local ok, allowed = pcall(previousGate, ped)
        if not ok or allowed == false then return false end
    end
    return true
end
RegisterNetEvent('gtaxskate:sk8m:state', function(incoming)
    if tonumber(source) ~= 65535 then return end
    if type(incoming) ~= 'table' or incoming.version ~= 1 or type(incoming.settings) ~= 'table' or
        type(incoming.generation) ~= 'string' or type(incoming.revision) ~= 'number' then
        Friends.invalidate()
        return
    end
    for _, key in ipairs({ 'PushSpeed', 'PushPower', 'Braking', 'Steering' }) do
        local n = incoming.settings[key]
        if type(n) ~= 'number' or n ~= n or n < 0.5 or n > 3 then Friends.invalidate() return end
    end
    local changed = not same(snapshot, incoming)
    snapshot = incoming
    pending, reply, grant = nil, nil, nil
    -- Rebuild from the immutable original, never multiply previously scaled values.
    Config.PushMax = baseline.PushMax * incoming.settings.PushSpeed
    Config.PushImpulse = baseline.PushImpulse * incoming.settings.PushPower
    Config.BrakeDecel = baseline.BrakeDecel * incoming.settings.Braking
    Config.TurnRate = baseline.TurnRate * incoming.settings.Steering
    Tune.reset()
    -- A new policy stops an existing rider; remount uses the new settings.
    -- This also cancels delayed model/item grants from the previous revision.
    if changed then Friends.stop() end
end)
RegisterNetEvent('skateboard:boardReply', function(ok, token, generation, revision)
    if tonumber(source) ~= 65535 or not pending or token ~= pending.token then return end
    if not same(snapshot, pending) or generation ~= pending.generation or revision ~= pending.revision then return end
    reply = ok == true
end)
function Friends.authorize()
    if not Friends.canRide() then return false end
    sequence = sequence + 1
    local request = { token = nonce .. ':' .. tostring(sequence), generation = snapshot.generation, revision = snapshot.revision }
    pending, reply = request, nil
    TriggerServerEvent('skateboard:requestBoard', request.token, request.generation, request.revision)
    local deadline = GetGameTimer() + 3000
    while pending == request and reply == nil and GetGameTimer() < deadline do Wait(0) end
    local yes = pending == request and reply == true and same(snapshot, request) and Friends.canRide()
    grant = yes and request or nil
    if pending == request then pending, reply = nil, nil end
    return yes
end
function Friends.granted() return Friends.canRide() and same(snapshot, grant) end
AddEventHandler('onClientResourceStop', function(resource)
    if resource == 'gtaxskate_rules' or resource == 'gtaxskate_sk8m' then Friends.invalidate() end
end)
AddEventHandler('onClientResourceStart', function(resource)
    if resource == GetCurrentResourceName() or resource == 'gtaxskate_rules' or resource == 'gtaxskate_sk8m' then
        Friends.invalidate()
        syncRemaining, syncAt = 15, GetGameTimer() + 1000
        TriggerServerEvent('gtaxskate:sk8m:sync')
    end
end)
-- Local lifecycle fallback only, no network polling or settings work per frame.
CreateThread(function()
    while true do
        Wait(250)
        if snapshot and not Friends.canRide() then Friends.invalidate() end
        if not snapshot and syncRemaining > 0 and GetGameTimer() >= syncAt and
            GetResourceState('gtaxskate_rules') == 'started' and GetResourceState('gtaxskate_sk8m') == 'started' then
            syncRemaining, syncAt = syncRemaining - 1, GetGameTimer() + 1000
            TriggerServerEvent('gtaxskate:sk8m:sync')
        end
    end
end)

-- Read-only developer diagnostics: never grant settings or bypass server policy.
RegisterCommand('skatedev', function()
    TriggerServerEvent('gtaxskate:sk8m:sync')
    TriggerServerEvent('gtaxskate:rules:request')
    print('[GTAxSkate] SK8M Lua fallback; Skate 3 physics not connected; multiplayer acceptance pending.')
    if not snapshot then print('[GTAxSkate] No valid server policy; mounting is blocked.') return end
    print(('[GTAxSkate] Policy generation %s revision %s; ready %s'):format(
        snapshot.generation, tostring(snapshot.revision), tostring(Friends.canRide())))
    for _, key in ipairs({ 'PushSpeed', 'PushPower', 'Braking', 'Steering' }) do
        print(('[GTAxSkate] %s = %s'):format(key, tostring(snapshot.settings[key])))
    end
end, false)

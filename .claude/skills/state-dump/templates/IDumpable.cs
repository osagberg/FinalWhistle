// Opt-in interface for runtime state-dump skill.
//
// Any MonoBehaviour that implements IDumpable will be discovered by
// McpRemoteControl.DumpState() during a scene sweep and its return value
// serialized under `components.<ComponentTypeName>` in StateDump.json.
//
// Return an anonymous object — the serializer (Newtonsoft.Json via
// com.unity.nuget.newtonsoft-json) handles anonymous types cleanly.
// Keep returns small: only fields Claude would use to reason about state.
// Don't dump the whole MonoBehaviour — PII of the scene graph is noise.

namespace FinalWhistle.Debug
{
    public interface IDumpable
    {
        object DumpState();
    }
}

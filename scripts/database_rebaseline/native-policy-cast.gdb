# QueryDesc.sourceText is at byte 16 in the qualified AArch64 PG 16.14 ABI.
python
import gdb
import json

class CastExecutorScope(gdb.Breakpoint):
    def __init__(self):
        super().__init__("*ExecutorEnd", type=gdb.BP_HARDWARE_BREAKPOINT,
                         internal=True)
        self.silent = True

    def stop(self):
        try:
            descriptor = int(gdb.parse_and_eval("$x0"))
            if descriptor == 0:
                return False
            source = gdb.parse_and_eval(
                "*(char **)(%#x + 16)" % descriptor)
            if int(source) == 0:
                return False
            query = source.string(encoding="utf-8", errors="surrogateescape")
            decision = ("INSERT INTO search_filter_decision" in query
                        and "FROM tmp_policy_matches" in query)
            if decision or "policy_helpers:" in query:
                event = {
                    "kind": "executor_end",
                    "query": query,
                    "compiler_setting": int(gdb.parse_and_eval(
                        "(int)plpgsql_variable_conflict")),
                    "backend": int(gdb.parse_and_eval("(int)MyProcPid")),
                    "descriptor": hex(descriptor),
                }
                gdb.write("CAST_EXECUTOR:" + json.dumps(
                    event, ensure_ascii=True, separators=(",", ":")) + "\n")
        except Exception as error:
            gdb.write("CAST_EXECUTOR_ERROR:" + str(error).replace("\n", " ")
                      + "\n", gdb.STDERR)
        return False

cast_executor_scope = CastExecutorScope()
end

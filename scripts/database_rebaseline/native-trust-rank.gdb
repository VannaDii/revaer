# Read-only observer for the exact LP64 PostgreSQL 16.14 target.
# Field offsets come from that target's retained plpgsql.h, not debugger types.
python
import gdb
import json

def k1_int(address, width="int"):
    return int(gdb.parse_and_eval("*(%s *)(%#x)" % (width, address)))

def k1_ptr(address):
    return k1_int(address, "unsigned long")

def k1_text(address):
    return gdb.parse_and_eval("(char *)(%#x)" % address).string(
        encoding="utf-8", errors="strict")

def k1_context(estate):
    function = k1_ptr(estate)
    signature = k1_text(k1_ptr(function))
    if not signature.startswith("search_result_ingest_v1("):
        return None
    count = k1_int(estate + 108)
    if not 0 < count < 1000:
        raise RuntimeError("invalid target datum count")
    datums = k1_ptr(estate + 112)
    wanted = {"instance_trust_tier_key", "instance_trust_rank", "trust_bucket"}
    values = {}
    for index in range(count):
        datum = k1_ptr(datums + index * 8)
        if k1_int(datum) != 0:
            continue
        name = k1_text(k1_ptr(datum + 8))
        if name not in wanted:
            continue
        if name in values or k1_int(datum + 4) != index:
            raise RuntimeError("ambiguous target datum identity")
        isnull = k1_int(datum + 64, "unsigned char")
        if isnull not in (0, 1):
            raise RuntimeError("invalid target null flag")
        values[name] = {"dno": index, "declaration_line": k1_int(datum + 16),
                        "isnull": bool(isnull)}
        if name != "instance_trust_tier_key":
            values[name]["value"] = None if isnull else k1_int(datum + 56, "long")
    if set(values) != wanted:
        raise RuntimeError("missing target local variables")
    statement = k1_ptr(estate + 208)
    return {"backend": int(gdb.parse_and_eval("(int)MyProcPid")),
            "function_oid": k1_int(function + 8, "unsigned int"),
            "signature": signature,
            "compiler_setting": int(gdb.parse_and_eval("(int)plpgsql_variable_conflict")),
            "resolve_option": k1_int(function + 484),
            "line": k1_int(statement + 4) if statement else None,
            "statement_id": k1_int(statement + 8, "unsigned int") if statement else None,
            "locals": values}

def k1_emit(event):
    gdb.write("K1:" + json.dumps(event, ensure_ascii=True,
                               separators=(",", ":")) + "\n")

def k1_error(error):
    gdb.write("K1_ERROR:" + str(error).replace("\n", " ") + "\n",
              gdb.STDERR)

class K1Return(gdb.FinishBreakpoint):
    def __init__(self, estate, event, boolean):
        super().__init__(gdb.newest_frame(), internal=True)
        self.silent = True
        self.estate = estate
        self.event = event
        self.boolean = boolean

    def stop(self):
        try:
            event = dict(self.event)
            event["phase"] = "return"
            event["after"] = k1_context(self.estate)
            if self.boolean:
                event["answer"] = int(gdb.parse_and_eval("$x0"))
            k1_emit(event)
        except Exception as error:
            k1_error(error)
        return False

    def out_of_scope(self):
        k1_error("target expression did not return")

class K1Entry(gdb.Breakpoint):
    def __init__(self, symbol, boolean):
        super().__init__("*" + symbol, internal=True)
        self.silent = True
        self.boolean = boolean

    def stop(self):
        try:
            estate = int(gdb.parse_and_eval("$x0"))
            context = k1_context(estate)
            if context is None:
                return False
            if self.boolean:
                expr = int(gdb.parse_and_eval("$x1"))
                query = k1_text(k1_ptr(expr))
                if query not in ("instance_trust_tier_key IS NOT NULL",
                                 "instance_trust_rank IS NULL",
                                 "instance_trust_rank >= 40",
                                 "instance_trust_rank >= 30",
                                 "instance_trust_rank >= 20"):
                    return False
                event = {"kind": "boolean", "query": query}
            else:
                target = int(gdb.parse_and_eval("$x1"))
                name = k1_text(k1_ptr(target + 8))
                if name not in ("instance_trust_rank", "trust_bucket"):
                    return False
                expr = int(gdb.parse_and_eval("$x2"))
                event = {"kind": "assignment", "target": name,
                         "query": k1_text(k1_ptr(expr))}
            event.update({"phase": "entry", "before": context})
            k1_emit(event)
            K1Return(estate, event, self.boolean)
        except Exception as error:
            k1_error(error)
        return False

k1_boolean = K1Entry("exec_eval_boolean", True)
k1_assignment = K1Entry("exec_assign_expr", False)
end

# Read-only qualified policy probes for AArch64 PostgreSQL 16.14.
break *textregexeq
commands
silent
printf "REGEX:textregexeq:%d\n", (int)plpgsql_variable_conflict
continue
end
break *texticregexeq
commands
silent
printf "REGEX:texticregexeq:%d\n", (int)plpgsql_variable_conflict
continue
end

# frozen_string_literal: true

module RevaerPostgresPristine
  # Complete PostgreSQL 16.14 column inventory, checked against pg_attribute before extraction.
  # This describes catalog serialization, not admissible provider objects or an application schema.
  COLUMNS = {
    "pg_namespace" => %w[oid nspname nspowner nspacl],
    "pg_extension" => %w[oid extname extowner extnamespace extrelocatable extversion extconfig extcondition],
    "pg_class" => %w[oid relname relnamespace reltype reloftype relowner relam relfilenode reltablespace relpages reltuples relallvisible reltoastrelid relhasindex relisshared relpersistence relkind relnatts relchecks relhasrules relhastriggers relhassubclass relrowsecurity relforcerowsecurity relispopulated relreplident relispartition relrewrite relfrozenxid relminmxid relacl reloptions relpartbound],
    "pg_proc" => %w[oid proname pronamespace proowner prolang procost prorows provariadic prosupport prokind prosecdef proleakproof proisstrict proretset provolatile proparallel pronargs pronargdefaults prorettype proargtypes proallargtypes proargmodes proargnames proargdefaults protrftypes prosrc probin prosqlbody proconfig proacl],
    "pg_type" => %w[oid typname typnamespace typowner typlen typbyval typtype typcategory typispreferred typisdefined typdelim typrelid typsubscript typelem typarray typinput typoutput typreceive typsend typmodin typmodout typanalyze typalign typstorage typnotnull typbasetype typtypmod typndims typcollation typdefaultbin typdefault typacl],
    "pg_trigger" => %w[oid tgrelid tgparentid tgname tgfoid tgtype tgenabled tgisinternal tgconstrrelid tgconstrindid tgconstraint tgdeferrable tginitdeferred tgnargs tgattr tgargs tgqual tgoldtable tgnewtable],
    "pg_event_trigger" => %w[oid evtname evtevent evtowner evtfoid evtenabled evttags],
    "pg_language" => %w[oid lanname lanowner lanispl lanpltrusted lanplcallfoid laninline lanvalidator lanacl],
    "pg_cast" => %w[oid castsource casttarget castfunc castcontext castmethod],
    "pg_collation" => %w[oid collname collnamespace collowner collprovider collisdeterministic collencoding collcollate collctype colliculocale collicurules collversion],
    "pg_conversion" => %w[oid conname connamespace conowner conforencoding contoencoding conproc condefault],
    "pg_operator" => %w[oid oprname oprnamespace oprowner oprkind oprcanmerge oprcanhash oprleft oprright oprresult oprcom oprnegate oprcode oprrest oprjoin],
    "pg_opclass" => %w[oid opcmethod opcname opcnamespace opcowner opcfamily opcintype opcdefault opckeytype],
    "pg_opfamily" => %w[oid opfmethod opfname opfnamespace opfowner],
    "pg_amop" => %w[oid amopfamily amoplefttype amoprighttype amopstrategy amoppurpose amopopr amopmethod amopsortfamily],
    "pg_amproc" => %w[oid amprocfamily amproclefttype amprocrighttype amprocnum amproc],
    "pg_ts_config" => %w[oid cfgname cfgnamespace cfgowner cfgparser],
    "pg_ts_dict" => %w[oid dictname dictnamespace dictowner dicttemplate dictinitoption],
    "pg_ts_parser" => %w[oid prsname prsnamespace prsstart prstoken prsend prsheadline prslextype],
    "pg_ts_template" => %w[oid tmplname tmplnamespace tmplinit tmpllexize],
    "pg_foreign_data_wrapper" => %w[oid fdwname fdwowner fdwhandler fdwvalidator fdwacl fdwoptions],
    "pg_foreign_server" => %w[oid srvname srvowner srvfdw srvtype srvversion srvacl srvoptions],
    "pg_user_mapping" => %w[oid umuser umserver umoptions],
    "pg_policy" => %w[oid polname polrelid polcmd polpermissive polroles polqual polwithcheck],
    "pg_publication" => %w[oid pubname pubowner puballtables pubinsert pubupdate pubdelete pubtruncate pubviaroot],
    "pg_publication_rel" => %w[oid prpubid prrelid prqual prattrs],
    "pg_subscription" => %w[oid subdbid subskiplsn subname subowner subenabled subbinary substream subtwophasestate subdisableonerr subpasswordrequired subrunasowner subconninfo subslotname subsynccommit subpublications suborigin]
  }.freeze

  EXCLUDED = {
    "relfilenode" => "filesystem relation storage identity",
    "reltablespace" => "filesystem tablespace location",
    "relpages" => "planner statistics", "reltuples" => "planner statistics",
    "relallvisible" => "visibility statistics", "relfrozenxid" => "transaction ID",
    "relminmxid" => "transaction ID"
  }.freeze

  REFERENCES = {
    "pg_namespace" => %w[extnamespace relnamespace pronamespace typnamespace collnamespace connamespace opcnamespace oprnamespace opfnamespace cfgnamespace dictnamespace prsnamespace tmplnamespace],
    "pg_authid" => %w[nspowner extowner relowner proowner typowner evtowner lanowner collowner conowner opcowner oprowner opfowner cfgowner dictowner fdwowner srvowner umuser polroles pubowner subowner],
    "pg_class" => %w[extconfig reltoastrelid relrewrite typrelid tgrelid tgconstrrelid tgconstrindid polrelid prrelid],
    "pg_type" => %w[reltype reloftype provariadic prorettype proargtypes proallargtypes protrftypes typelem typarray typbasetype castsource casttarget opcintype opckeytype oprleft oprright oprresult amoplefttype amoprighttype amproclefttype amprocrighttype],
    "pg_proc" => %w[prosupport typsubscript typinput typoutput typreceive typsend typmodin typmodout typanalyze tgfoid evtfoid lanplcallfoid laninline lanvalidator castfunc conproc oprcode oprrest oprjoin amproc prsstart prstoken prsend prsheadline prslextype tmplinit tmpllexize fdwhandler fdwvalidator],
    "pg_am" => %w[relam opcmethod opfmethod amopmethod],
    "pg_language" => %w[prolang], "pg_collation" => %w[typcollation],
    "pg_trigger" => %w[tgparentid], "pg_constraint" => %w[tgconstraint],
    "pg_opfamily" => %w[opcfamily amopfamily amopsortfamily amprocfamily],
    "pg_operator" => %w[oprcom oprnegate amopopr], "pg_ts_parser" => %w[cfgparser],
    "pg_ts_template" => %w[dicttemplate], "pg_foreign_data_wrapper" => %w[srvfdw],
    "pg_foreign_server" => %w[umserver], "pg_publication" => %w[prpubid], "pg_database" => %w[subdbid]
  }.flat_map { |catalog, columns| columns.map { |column| [column, catalog] } }.to_h.freeze

  EXPRESSIONS = {
    "relpartbound" => "pg_get_expr(t.relpartbound, t.oid, false)",
    "proargdefaults" => "pg_get_expr(t.proargdefaults, 0, false)",
    "prosqlbody" => "CASE WHEN t.prosqlbody IS NOT NULL THEN pg_get_functiondef(t.oid) END",
    "typdefaultbin" => "pg_get_expr(t.typdefaultbin, 0, false)",
    "tgqual" => "CASE WHEN t.tgqual IS NOT NULL THEN pg_get_triggerdef(t.oid, false) END",
    "polqual" => "pg_get_expr(t.polqual, t.polrelid, false)",
    "polwithcheck" => "pg_get_expr(t.polwithcheck, t.polrelid, false)",
    "prqual" => "pg_get_expr(t.prqual, t.prrelid, false)"
  }.freeze
end

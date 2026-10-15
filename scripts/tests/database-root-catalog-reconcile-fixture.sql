-- Runtime-role populated reconciliation. Outer test transaction is rolled back.
DO $$
DECLARE
    candidate record;
    slot_id uuid;
    activated record;
BEGIN
    SELECT * INTO STRICT candidate FROM public.media_root_catalog_reconcile_begin_v1(
        1::smallint,
        decode('121ea684c73474764ce8f45fe5bc9294977d17bc540cc485dcf433fe5e4a2945', 'hex'),
        decode('d46abdc7f552b03a9e41bbcfdddd8e8cd17e7462c515fc801e144701456918d2', 'hex'),
        decode('c03388a6b0e9600eee95bf451ddd9f58c4f75f2c2700827dc06a5821c1e7b60f', 'hex'),
        2::smallint);
    IF candidate.already_current THEN RAISE EXCEPTION 'unexpected current populated fixture'; END IF;
    slot_id := public.media_root_catalog_reconcile_slot_v1(
        candidate.media_root_catalog_generation_public_id, 'a', U&'/m/\00E9', U&'/real/\00E9',
        decode('0102030405060708', 'hex'), decode('ffffffffffffffff', 'hex'), 0, 'ext4',
        true, false, false, false, false, false, false,
        'disposable', 'none', 'uncontrolled', 'none', 0, 4294967295, 4095,
        decode('22595f1988e86623ad0b03c3ce791a0792a721808dd3d57e81585f1955e2bc4f', 'hex'));
    PERFORM public.media_root_catalog_reconcile_slot_kind_v1(candidate.media_root_catalog_generation_public_id, slot_id, 'source');
    slot_id := public.media_root_catalog_reconcile_slot_v1(
        candidate.media_root_catalog_generation_public_id, 'z9', '/work', '/work',
        decode('8000000000000000', 'hex'), decode('0000000000000000', 'hex'), 9223372036854775807, 'xfs',
        false, true, true, true, true, true, true,
        'restart_persistent', 'kubernetes_persistent_volume_claim', 'revaer_exclusive',
        'kubernetes_read_write_once_pod', 4294967295, 2147483648, 0,
        decode('cfdfe871f1d7a72b088aef25782ea79eef3ae26df61d9848c08a663b25ee4fde', 'hex'));
    PERFORM public.media_root_catalog_reconcile_slot_kind_v1(candidate.media_root_catalog_generation_public_id, slot_id, 'workspace');
    SELECT * INTO STRICT activated FROM public.media_root_catalog_reconcile_activate_v1(candidate.media_root_catalog_generation_public_id);
    IF activated.slot_count <> 2 OR activated.attestation_generation <> candidate.attestation_generation THEN
        RAISE EXCEPTION 'populated activation returned a different generation';
    END IF;
    IF (SELECT count(*) FROM public.media_root_catalog_slot_page_v1(50::smallint, NULL, NULL)) <> 2 THEN
        RAISE EXCEPTION 'activated populated catalog was not readable';
    END IF;
END;
$$;

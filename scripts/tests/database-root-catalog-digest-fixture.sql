-- Exact ADR 557 vectors shared with the independent Rust identity tests.
-- Run only inside an owned test transaction that the caller rolls back.
DO $$
DECLARE
    generation_id bigint;
    empty_generation_id bigint;
    observed record;
BEGIN
    INSERT INTO public.media_root_catalog_generation
        (contract_version, source_format_version, source_sha256, attestation_sha256,
         generation_sha256, slot_count, activated_at)
    VALUES (1, 1,
        decode('121ea684c73474764ce8f45fe5bc9294977d17bc540cc485dcf433fe5e4a2945', 'hex'),
        decode('d46abdc7f552b03a9e41bbcfdddd8e8cd17e7462c515fc801e144701456918d2', 'hex'),
        decode('c03388a6b0e9600eee95bf451ddd9f58c4f75f2c2700827dc06a5821c1e7b60f', 'hex'),
        2, transaction_timestamp())
    RETURNING media_root_catalog_generation_id INTO generation_id;
    -- Reverse insertion order proves identity is not heap/identity order.
    INSERT INTO public.media_root_catalog_slot (logical_key) VALUES ('z9'), ('a');
    INSERT INTO public.media_root_catalog_slot_attestation
        (media_root_catalog_generation_id, media_root_catalog_slot_id,
         requested_path, canonical_path, filesystem_device, filesystem_inode,
         mount_id, filesystem_type, read_capable, write_capable, create_new_capable,
         fsync_capable, rename_capable, delete_capable, capacity_probe_capable,
         durability_class, durability_evidence, sole_writer_class, sole_writer_evidence,
         owner_uid, owner_gid, mode_bits, validated_at, root_identity_sha256)
    SELECT generation_id, s.media_root_catalog_slot_id,
        CASE s.logical_key WHEN 'a' THEN U&'/m/\00E9' ELSE '/work' END,
        CASE s.logical_key WHEN 'a' THEN U&'/real/\00E9' ELSE '/work' END,
        decode(CASE s.logical_key WHEN 'a' THEN '0102030405060708' ELSE '8000000000000000' END, 'hex'),
        decode(CASE s.logical_key WHEN 'a' THEN 'ffffffffffffffff' ELSE '0000000000000000' END, 'hex'),
        CASE s.logical_key WHEN 'a' THEN 0 ELSE 9223372036854775807 END,
        CASE s.logical_key WHEN 'a' THEN 'ext4' ELSE 'xfs' END,
        s.logical_key = 'a', s.logical_key = 'z9', s.logical_key = 'z9',
        s.logical_key = 'z9', s.logical_key = 'z9', s.logical_key = 'z9', s.logical_key = 'z9',
        CASE s.logical_key WHEN 'a' THEN 'disposable' ELSE 'restart_persistent' END,
        CASE s.logical_key WHEN 'a' THEN 'none' ELSE 'kubernetes_persistent_volume_claim' END,
        CASE s.logical_key WHEN 'a' THEN 'uncontrolled' ELSE 'revaer_exclusive' END,
        CASE s.logical_key WHEN 'a' THEN 'none' ELSE 'kubernetes_read_write_once_pod' END,
        CASE s.logical_key WHEN 'a' THEN 0 ELSE 4294967295 END,
        CASE s.logical_key WHEN 'a' THEN 4294967295 ELSE 2147483648 END,
        CASE s.logical_key WHEN 'a' THEN 4095 ELSE 0 END,
        transaction_timestamp(),
        decode(CASE s.logical_key
            WHEN 'a' THEN '22595f1988e86623ad0b03c3ce791a0792a721808dd3d57e81585f1955e2bc4f'
            ELSE 'cfdfe871f1d7a72b088aef25782ea79eef3ae26df61d9848c08a663b25ee4fde' END, 'hex')
    FROM public.media_root_catalog_slot AS s WHERE s.logical_key IN ('a', 'z9');
    INSERT INTO public.media_root_catalog_slot_kind
        (media_root_catalog_slot_attestation_id, media_root_kind_id)
    SELECT a.media_root_catalog_slot_attestation_id,
        CASE s.logical_key WHEN 'a' THEN 1 ELSE 3 END
    FROM public.media_root_catalog_slot_attestation AS a
    JOIN public.media_root_catalog_slot AS s USING (media_root_catalog_slot_id)
    WHERE a.media_root_catalog_generation_id = generation_id;
    SELECT * INTO STRICT observed FROM public.media_root_catalog_digests_v1(generation_id);
    IF encode(observed.source_sha256, 'hex') <> '121ea684c73474764ce8f45fe5bc9294977d17bc540cc485dcf433fe5e4a2945'
       OR encode(observed.attestation_sha256, 'hex') <> 'd46abdc7f552b03a9e41bbcfdddd8e8cd17e7462c515fc801e144701456918d2'
       OR encode(observed.generation_sha256, 'hex') <> 'c03388a6b0e9600eee95bf451ddd9f58c4f75f2c2700827dc06a5821c1e7b60f' THEN
        RAISE EXCEPTION 'root catalog populated digest vector mismatch';
    END IF;
    BEGIN
        INSERT INTO public.media_root_catalog_slot_kind
            (media_root_catalog_slot_attestation_id, media_root_kind_id)
        SELECT a.media_root_catalog_slot_attestation_id, 2
        FROM public.media_root_catalog_slot_attestation AS a
        JOIN public.media_root_catalog_slot AS s USING (media_root_catalog_slot_id)
        WHERE a.media_root_catalog_generation_id = generation_id AND s.logical_key = 'a';
        PERFORM * FROM public.media_root_catalog_digests_v1(generation_id);
        RAISE EXCEPTION 'changed kind was accepted with the original slot digest';
    EXCEPTION WHEN SQLSTATE 'P0001' THEN
        IF SQLERRM <> 'media_root_attestation_invalid' THEN RAISE; END IF;
    END;
    -- The failed subtransaction must not leave its extra kind behind.
    SELECT * INTO STRICT observed FROM public.media_root_catalog_digests_v1(generation_id);
    IF encode(observed.generation_sha256, 'hex') <> 'c03388a6b0e9600eee95bf451ddd9f58c4f75f2c2700827dc06a5821c1e7b60f' THEN
        RAISE EXCEPTION 'rejected kind mutation was not rolled back';
    END IF;
    INSERT INTO public.media_root_catalog_generation
        (contract_version, source_format_version, source_sha256, attestation_sha256,
         generation_sha256, slot_count, activated_at)
    VALUES (1, 1,
        decode('8f0b256ad26c139e8c51bd426ba510eb43aefca9cb1d58fb0737b6518e6bc867', 'hex'),
        decode('8f02f1258c4d32e0a9919e632d0ab9ec9c17a96f888ee26ee90fb8b9007568cd', 'hex'),
        decode('c69e6c6c3b5624cb119e895a8dac30a7989143139f9bde93199b8268295e2bc4', 'hex'),
        0, transaction_timestamp())
    RETURNING media_root_catalog_generation_id INTO empty_generation_id;
    SELECT * INTO STRICT observed FROM public.media_root_catalog_digests_v1(empty_generation_id);
    IF encode(observed.source_sha256, 'hex') <> '8f0b256ad26c139e8c51bd426ba510eb43aefca9cb1d58fb0737b6518e6bc867'
       OR encode(observed.attestation_sha256, 'hex') <> '8f02f1258c4d32e0a9919e632d0ab9ec9c17a96f888ee26ee90fb8b9007568cd'
       OR encode(observed.generation_sha256, 'hex') <> 'c69e6c6c3b5624cb119e895a8dac30a7989143139f9bde93199b8268295e2bc4' THEN
        RAISE EXCEPTION 'root catalog empty digest vector mismatch';
    END IF;
END;
$$;

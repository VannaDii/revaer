-- Synthetic persisted evidence for reader tests, not filesystem attestation proof.
DO $$
DECLARE
    generation_id bigint;
    slot_id bigint;
    attestation_id bigint;
    slot_number integer;
BEGIN
    INSERT INTO public.media_root_catalog_generation (
        contract_version, source_format_version, source_sha256, attestation_sha256,
        generation_sha256, slot_count, activated_at
    ) VALUES (1, 1, decode(repeat('11', 32), 'hex'), decode(repeat('22', 32), 'hex'),
        decode(repeat('33', 32), 'hex'), 2, transaction_timestamp())
    RETURNING media_root_catalog_generation_id INTO generation_id;
    FOR slot_number IN 1..2 LOOP
        INSERT INTO public.media_root_catalog_slot (logical_key)
        VALUES ('reader-' || slot_number)
        RETURNING media_root_catalog_slot_id INTO slot_id;
        INSERT INTO public.media_root_catalog_slot_attestation (
            media_root_catalog_generation_id, media_root_catalog_slot_id,
            requested_path, canonical_path, filesystem_device, filesystem_inode,
            mount_id, filesystem_type, read_capable, write_capable, create_new_capable,
            fsync_capable, rename_capable, delete_capable, capacity_probe_capable,
            durability_class, durability_evidence, sole_writer_class, sole_writer_evidence,
            owner_uid, owner_gid, mode_bits, validated_at, root_identity_sha256
        ) VALUES (
            generation_id, slot_id, '/reader-fixture/' || slot_number, '/reader-fixture/' || slot_number,
            decode('0000000000000001', 'hex'), int8send(slot_number::bigint),
            1, 'fixture', true, true, true, true, true, true, true,
            'restart_persistent', 'linux_dedicated_mount', 'revaer_exclusive', 'linux_dedicated_service',
            1000, 1000, 448, transaction_timestamp(), decode(repeat(slot_number::text, 64), 'hex')
        ) RETURNING media_root_catalog_slot_attestation_id INTO attestation_id;
        INSERT INTO public.media_root_catalog_slot_kind
        VALUES (attestation_id, CASE WHEN slot_number = 1 THEN 1 ELSE 3 END);
        IF slot_number = 1 THEN
            INSERT INTO public.media_root_catalog_slot_kind VALUES (attestation_id, 2);
        END IF;
    END LOOP;
    UPDATE public.media_root_catalog_state
    SET active_media_root_catalog_generation_id = generation_id,
        source_state = 'ready', source_reason_code = NULL,
        attestation_state = 'ready', attestation_reason_code = NULL,
        reconciled_at = transaction_timestamp()
    WHERE media_root_catalog_state_id = 1;
END;
$$;

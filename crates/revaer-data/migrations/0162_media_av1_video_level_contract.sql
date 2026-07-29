CREATE OR REPLACE FUNCTION media_video_level_known_v1(
    codec_input TEXT,
    video_level_input TEXT
)
RETURNS BOOLEAN
LANGUAGE sql
IMMUTABLE
PARALLEL SAFE
AS $$
    SELECT CASE
        WHEN NULLIF(lower(btrim(video_level_input)), '') IS NULL THEN TRUE
        WHEN lower(btrim(codec_input)) IN ('h264', 'avc', 'avc1', 'libx264', 'x264') THEN
            lower(btrim(video_level_input)) IN (
                '1', '1.0', '10', '1b',
                '1.1', '11', '1.2', '12', '1.3', '13',
                '2', '2.0', '20', '2.1', '21', '2.2', '22',
                '3', '3.0', '30', '3.1', '31', '3.2', '32',
                '4', '4.0', '40', '4.1', '41', '4.2', '42',
                '5', '5.0', '50', '5.1', '51', '5.2', '52',
                '6', '6.0', '60', '6.1', '61', '6.2', '62'
            )
        WHEN lower(btrim(codec_input)) IN ('hevc', 'h265', 'libx265', 'x265') THEN
            lower(btrim(video_level_input)) IN (
                '1', '1.0', '10',
                '2', '2.0', '20', '2.1', '21',
                '3', '3.0', '30', '3.1', '31',
                '4', '4.0', '40', '4.1', '41',
                '5', '5.0', '50', '5.1', '51', '5.2', '52',
                '6', '6.0', '60', '6.1', '61', '6.2', '62'
            )
        WHEN lower(btrim(codec_input)) IN (
            'av1',
            'av01',
            'libaom-av1',
            'librav1e',
            'libsvtav1',
            'libsvt-av1'
        ) THEN
            lower(btrim(video_level_input)) IN (
                '2', '2.0', '20', '2.1', '21', '2.2', '22', '2.3', '23',
                '3', '3.0', '30', '3.1', '31', '3.2', '32', '3.3', '33',
                '4', '4.0', '40', '4.1', '41', '4.2', '42', '4.3', '43',
                '5', '5.0', '50', '5.1', '51', '5.2', '52', '5.3', '53',
                '6', '6.0', '60', '6.1', '61', '6.2', '62', '6.3', '63',
                '7', '7.0', '70', '7.1', '71', '7.2', '72', '7.3', '73'
            )
        ELSE FALSE
    END
$$;

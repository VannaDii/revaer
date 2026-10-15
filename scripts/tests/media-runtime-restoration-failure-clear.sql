-- Remove the owned fixture fault so retained recovery can retry.
DROP TRIGGER fixture_restoration_failure ON public.media_job;
DROP FUNCTION public.fixture_restoration_failure();

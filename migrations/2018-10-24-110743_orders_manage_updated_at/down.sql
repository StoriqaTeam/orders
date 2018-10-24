-- Do nothing
DROP TRIGGER IF EXISTS "set_updated_at" ON "orders";

DROP OPERATOR CLASS IF EXISTS point_hash_ops USING hash;

DROP FUNCTION IF EXISTS public.hashpoint;
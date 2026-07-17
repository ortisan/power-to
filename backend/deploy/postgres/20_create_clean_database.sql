-- The docker-postgis image loads PostGIS, topology, and tiger/geocoder objects
-- into POSTGRES_DB before this script runs. PowerTo migrations must start from
-- an otherwise clean database, so create the application database explicitly
-- from template0. Atlas will enable only the extension(s) it owns.
CREATE DATABASE powerto
    WITH OWNER powerto
    TEMPLATE template0
    ENCODING 'UTF8';

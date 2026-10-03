SELECT format('CREATE DATABASE %I TEMPLATE template0 ENCODING %L LC_COLLATE %L LC_CTYPE %L',
              :'database_name', 'UTF8', 'C', 'C') \gexec

# This is a BAD example, we should not store ssl certs in the container,
# instead use encrypted docker network, store secrets on the volume,
# or run PostgreSQL on the host without containerization.
# This example exists only as reminder how to configure ssl certs for PostgreSQL.

FROM postgres:16.1-alpine3.18

COPY ./center/out/postgresdb.key /var/lib/postgresql
COPY ./center/out/postgresdb.crt /var/lib/postgresql

COPY ./center/out/myCA.crt /var/lib/postgresql
COPY ./center/out/myCA.crl /var/lib/postgresql

COPY ./ssl-conf.sh /usr/local/bin/ssl-conf.sh
RUN chmod +x /usr/local/bin/ssl-conf.sh 

RUN chown 0:70 /var/lib/postgresql/postgresdb.key && chmod 640 /var/lib/postgresql/postgresdb.key
RUN chown 0:70 /var/lib/postgresql/postgresdb.crt && chmod 640 /var/lib/postgresql/postgresdb.crt

RUN chown 0:70 /var/lib/postgresql/myCA.crt && chmod 640 /var/lib/postgresql/myCA.crt
RUN chown 0:70 /var/lib/postgresql/myCA.crl && chmod 640 /var/lib/postgresql/myCA.crl

ENTRYPOINT ["bash", "/usr/local/bin/ssl-conf.sh"] 

CMD [ "-c", "ssl=on" , "-c", "ssl_cert_file=/var/lib/postgresql/postgresdb.crt", "-c",\
    "ssl_key_file=/var/lib/postgresql/postgresdb.key", "-c",\
    "ssl_ca_file=/var/lib/postgresql/myCA.crt", "-c", "ssl_crl_file=/var/lib/postgresql/myCA.crl" ]

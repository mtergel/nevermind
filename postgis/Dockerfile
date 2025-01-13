FROM postgres:17.2

RUN apt-get update \
    && apt-get install -y \
       postgresql-17-postgis-3 \
       postgis \
       && rm -rf /var/lib/apt/lists/*

ENV POSTGRES_USER=postgres
ENV POSTGRES_PASSWORD=password
ENV POSTGRES_DB=nevermind

EXPOSE 5432

CMD ["postgres"]

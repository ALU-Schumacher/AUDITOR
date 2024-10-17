
There needs to be a running Postgres database.
Migration will be handled automatically if AUDITOR is given admin privileges.
If not, create a new user and database for AUDITOR to use and grant privileges using:
```
create database auditor;
create user auditor with encrypted password 'super_safe';
grant all privileges on database auditor to auditor;
\c auditor
GRANT ALL ON SCHEMA public TO auditor;
```


A separate Prometheus will need access to the kubelets.

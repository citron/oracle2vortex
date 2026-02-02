use anyhow::Result;
use tokio::io::AsyncWriteExt;
use tokio::process::Command as TokioCommand;

pub struct SqlclConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub sid: String,
    pub sqlcl_path: String,
    pub thick: bool,
}

pub struct SqlclProcess {
    pub child: tokio::process::Child,
}

impl SqlclProcess {
    pub async fn spawn(config: &SqlclConfig, sql_query: &str) -> Result<Self> {
        tracing::info!("Launching SQLcl process");

        let mut child = TokioCommand::new(&config.sqlcl_path)
            .arg("/nolog")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()?;

        // Send commands to SQLcl via stdin
        if let Some(mut stdin) = child.stdin.take() {
            // Connect with user/password@//host:port/sid format in one command
            let full_connect = format!("CONNECT {}/{}@//{}:{}/{}\n", 
                config.user,
                config.password,
                config.host, 
                config.port, 
                config.sid
            );
            stdin.write_all(full_connect.as_bytes()).await?;

            // Set driver mode if thick is requested
            if config.thick {
                tracing::info!("Enabling Oracle Thick driver");
                stdin.write_all(b"SET DRIVER THICK\n").await?;
            }

            // Optimisation de l'affichage pour l'export fichier
            stdin.write_all(b"SET FEEDBACK OFF\n").await?;      // Masque "X rows selected"
            stdin.write_all(b"SET TIMING OFF\n").await?;        // Masque le temps d'exécution
            stdin.write_all(b"SET VERIFY OFF\n").await?;        // Masque la substitution de variables
            stdin.write_all(b"SET HEADING OFF\n").await?;       // Pas d'en-tête (géré par JSON)
            stdin.write_all(b"SET PAGESIZE 0\n").await?;        // Pas de pagination
            stdin.write_all(b"SET TERMOUT OFF\n").await?;       // N'affiche rien à l'écran
            stdin.write_all(b"SET TRIMSPOOL ON\n").await?;      // Supprime les espaces en fin de ligne
            stdin.write_all(b"SET ENCODING UTF-8\n").await?;    // Force l'UTF-8

            // Configuration Régionale (Session) pour compatibilité JSON stricte
            stdin.write_all(b"ALTER SESSION SET NLS_NUMERIC_CHARACTERS = '.,';\n").await?;
            stdin.write_all(b"ALTER SESSION SET NLS_DATE_FORMAT = 'YYYY-MM-DD\"T\"HH24:MI:SS';\n").await?;
            stdin.write_all(b"ALTER SESSION SET NLS_TIMESTAMP_FORMAT = 'YYYY-MM-DD\"T\"HH24:MI:SS.FF';\n").await?;
            stdin.write_all(b"ALTER SESSION SET NLS_TIMESTAMP_TZ_FORMAT = 'YYYY-MM-DD\"T\"HH24:MI:SS.FF TZH:TZM';\n").await?;

            // Configuration du format de sortie SQLcl
            stdin.write_all(b"SET SQLFORMAT JSON\n").await?;

            // Execute the query (ensure it ends with semicolon)
            stdin.write_all(sql_query.as_bytes()).await?;
            if !sql_query.trim().ends_with(';') {
                stdin.write_all(b";").await?;
            }
            stdin.write_all(b"\n").await?;

            // Exit
            stdin.write_all(b"EXIT\n").await?;
            stdin.flush().await?;
        }

        tracing::info!("SQLcl process spawned successfully");

        Ok(Self { child })
    }

    pub fn stdout(&mut self) -> Option<tokio::process::ChildStdout> {
        self.child.stdout.take()
    }

    pub async fn wait(&mut self) -> Result<std::process::ExitStatus> {
        Ok(self.child.wait().await?)
    }
}

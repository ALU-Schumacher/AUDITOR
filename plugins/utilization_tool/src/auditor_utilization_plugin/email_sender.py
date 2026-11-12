import os
import smtplib
import ssl
from email.mime.multipart import MIMEMultipart
from email.mime.text import MIMEText
from logging import Logger

import pandas as pd
from dotenv import load_dotenv

load_dotenv()


async def send_email(
    smtp_server: str,
    smtp_port: int,
    df_sum: pd.DataFrame,
    logger: Logger,
    use_tls: bool = False,
) -> None:
    try:
        sender_email = os.getenv("SENDER_EMAIL")
        app_password = os.getenv("PASSWORD")
        receiver_email = os.getenv("RECEIVER_EMAIL")

        if not sender_email or not receiver_email:
            raise ValueError("SENDER_EMAIL and RECEIVER_EMAIL must be non-empty")

        html_table = df_sum.to_html(index=False, border=0, justify="center")

        subject = "Monthly Utilisation Report"
        body_html = f"""
        <html>
          <body>
            <h2>Monthly Utilisation Summary</h2>
            {html_table}
          </body>
        </html>
        """

        message = MIMEMultipart("alternative")
        message["Subject"] = subject
        message["From"] = sender_email
        message["To"] = receiver_email

        message.attach(MIMEText(body_html, "html"))

        with smtplib.SMTP(smtp_server, smtp_port) as server:
            if use_tls:
                server.starttls(context=ssl.create_default_context())
                server.ehlo()
            if app_password:
                server.login(sender_email, app_password)
            server.send_message(message)

        logger.info("Email with DataFrame sent successfully!")

    except smtplib.SMTPAuthenticationError:
        logger.exception("SMTP authentication failed. Check email or app password")

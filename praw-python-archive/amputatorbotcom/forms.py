from flask_wtf import FlaskForm
from wtforms import SubmitField, TextAreaField, SelectField
from wtforms.validators import DataRequired


class InputForm(FlaskForm):
    query = TextAreaField('Paste the URL of the AMP page, or the text that contains it, below:',
                          validators=[DataRequired()],
                          render_kw={"placeholder": "https://example.eu/amp/.."})
    generate_comment = SelectField("Generate a comment to post on Reddit",
                                   choices=[("true", "Enabled"), ("false", "Disabled")], default="false")
    use_gac = SelectField("Guess-and-check if necessary",
                          choices=[("true", "Enabled (recommended)"), ("false", "Disabled")], default="true")
    max_depth = SelectField("Maximum number of referrals to follow",
                            choices=[(0, "None"), (1, 1), (2, 2), (3, "3 (recommended)"), (4, 4)], default=3,
                            coerce=int)
    should_redirect = SelectField("Forward me to the first found canonical",
                                  choices=[("true", "Enabled"), ("false", "Disabled")], default="false")
    submit = SubmitField('Submit URL', render_kw={"onclick": "updateParams()"})

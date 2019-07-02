# iafisher/popquiz
A command-line program to take quizzes.

Install and run the popquiz application with (you must have Rust and Cargo installed):

```shell
$ git clone https://github.com/iafisher/popquiz.git
$ cd popquiz
$ cargo run -- edit <name>
```

The last command will open up an editor for you to create a new quiz. Follow the format described in the section below. When done, check out these commands:

```shell
# Take a quiz.
$ cargo run -- take <name>

# Count the questions in a quiz.
$ cargo run -- count <name>

# See previous results for a quiz.
$ cargo run -- results <name>

# Edit a quiz.
$ cargo run -- edit <name>

# Delete a quiz.
$ cargo run -- delete <name>
```


## Quiz file format
You create the quizzes yourself as JSON files. This section documents the format of the JSON, although note that since popquiz is under active development, the format may change in backwards-incompatible fashion without warning.

The root JSON object should have a single `questions` field mapping to an array of question objects:

```
{
  "questions": [
    ...
  ]
}
```

The format of the question objects depends on the kind of questions. popquiz currently supports four question types:

- Short answer questions
- Unordered list questions
- Ordered list questions
- Multiple-choice questions


## Short answer questions
```json
{
  "kind": "ShortAnswer",
  "text": "Which English countess is regarded as the first computer programmer?",
  "answer": "Ada Lovelace"
}
```

The `kind` field defaults to `"ShortAnswer"` so it is optional here. The `text` field is the text of the question, and the `answer` field is the correct answer, as a string.


## Unordered list questions
These are questions for which the quiz-taker must supply a list of answers, in any order.

```json
{
  "kind": "ListAnswer",
  "text": [
    "Name the four Home Islands of Japan.",
    "What are the four principal islands of the Japanese archipelago?"
  ],
  "answer_list": ["Hokkaido", "Honshu", "Shikoku", "Kyushu"]
}
```

Unordered list questions use an `answer_list` field instead of an `answer` field.


## Ordered list questions
These are questions for which the quiz-taker must supply a list of answers in a specified order.

```json
{
  "kind": "OrderedListAnswer",
  "text": "Who were the first three Presidents of the United States, in order?",
  "answer_list": [
    "George Washington",
    "John Adams",
    "Thomas  Jefferson"
  ]
}
```

The format of ordered list questions is almost the same as for unordered list questions, except that the order of `answer_list` is significant.


## Multiple-choice questions
```json
{
  "kind": "MultipleChoice",
  "text": "In what year did the Russo-Japanese War end?",
  "candidates": ["1878", "1945", "1918", "1908"],
  "answer": "1905"
}
```

The `candidates` field is for the incorrect answers to be displayed as options. It should **not** contain the correct answer, which goes in the `answer` field.


## Ungraded questions
```json
{
  "kind": "Ungraded",
  "text": "Describe the late medieval period in England.",
  "answer": "The late medieval period in England was an era of domestic turmoil and recurring war abroad in France. Beginning in the reign of the unstable Henry VI of the House of Lancaster, the legitimacy of the Lancastrian monopoly..."
}
```

For ungraded questions, popquiz will prompt for an answer, but it will not check the user's response, and the question will not count towards either the total correct or total incorrect for the quiz. After the user enters her answer, the text in the `answer` field will be displayed as a sample correct answer. The `Ungraded` kind is intended for long-answer questions which could not reasonably be graded automatically.

## Other fields
The following notes apply to all question types.

The `text` field may be an array of strings, to allow for multiple wordings of the same question.

In the `answer` and `answer_list` fields, an array of strings may be used instead of a single string, for multiple acceptable variants of the same answer.

Questions may have a `tags` field, which should be a list of strings. Tagged questions can be filtered using the command-line `--tag` and `--exclude` options.

Questions may have an `explanations` field, which should be a map from strings to strings. It is used to provide explanations for incorrect answers. For example, if a question had the following as its `explanations` field:

```json
"explanations": {
  "charleston": "Charleston is the capital of West Virginia, not South Carolina."
}
```

then if a user answered the question with "Charleston", the given message would be printed. The keys **must** be lowercase.

Questions may have an `id` field with a unique string value. The purpose of this field is to support another optional field, `depends`. If question A has `depends` set to `"some-id"`, and question B's `id` field is `"some-id"`, then question A will always be asked after question B.

**Note**: Currently the dependency resolver is not very sophisticated, so for the time being the following constraints hold:

- A question may only declare one dependency. If you provide a list of strings instead of a string in the `depends` field, a JSON parse error will occur.
- A question may only be involved in one dependence relation, so if question A depends on question B, then question B may not depend on any other question, and no other question may depend on question B. If you violate this constraint, no error will occur, but the question ordering algorithm may or may not produce an order that respects your dependencies. Future versions of popquiz may eliminate this constraint.


For a complete example of a quiz file, see `sample.json` in the root of this repository.

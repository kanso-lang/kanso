# Release notes

1. Parser
   - block quotes now nest:

     > outer
     > > inner, with a list:
     > > - one
     > > - two
   - fenced code inside items keeps its indent:

     ```kanso
     fn main
       print "hi"
     ```

2. Renderer
   * tight lists stay tight
   * loose ones get paragraphs

     even with a second paragraph

10) a list that starts at ten
11) and keeps counting

- [ ] not a task list, just brackets
-
  empty first line, content below

> lazy continuation
of a quote paragraph
- and a list that interrupts it

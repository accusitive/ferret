struct Stack {
  int data[1024];
  int top;
};
void stack_push(struct Stack *stack, int value);
int stack_peek(struct Stack *stack);
int stack_pop(struct Stack *stack);
struct Stack *stack_new();


struct VariableStore {
  int data[1024]
};

void varstore_set(struct VariableStore* store, int index, int data);
int varstore_get(struct VariableStore* store, int index);
struct VariableStore* varstore_new() ;

void println(int data) ;